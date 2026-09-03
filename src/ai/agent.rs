// AI Agent: implements ask / do / plan / build using AiProvider + tools + RAG
// ReAct-style loop with simple text tool calling format for broad model compatibility.

use crate::ai::{create_provider, AiConfig, AiError, AiProvider};
use crate::rag::RagEngine;
use crate::storage::LocalStorage;
use crate::tools::{execute_tool, get_tool_definitions};
use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCommand {
    Ask,
    Do,
    Plan,
    Build,
}

impl AiCommand {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim_start_matches('/').to_ascii_lowercase().as_str() {
            "ask" => Some(Self::Ask),
            "do" => Some(Self::Do),
            "plan" => Some(Self::Plan),
            "build" => Some(Self::Build),
            _ => None,
        }
    }

    pub fn max_steps(&self) -> usize {
        match self {
            AiCommand::Ask => 1,
            AiCommand::Do => 10,
            AiCommand::Plan => 2,
            AiCommand::Build => 12,
        }
    }

    fn system_prompt(&self) -> &'static str {
        match self {
            AiCommand::Ask => {
                "You are a helpful assistant in a terminal shell. Answer the user's question concisely.\n\
                 You can use tools via the exact format:\n\
                 TOOL: tool_name\n\
                 ARGS: {\"arg\": \"value\"}\n\
                 Stop after one tool call if needed; the system will give you the Observation.\n\
                 When you have the final answer, just write it normally without the TOOL prefix."
            }
            AiCommand::Do => {
                "You are an agent that executes terminal tasks using tools.\n\
                 Available tools: cd, touch, ls, cat, write_file, mkdir, delete_path, copy_path, move_path, grep, exec_cmd.\n\
                 Execute the requested task step-by-step using the appropriate tools.\n\
                 If the user asks for sequential operations (e.g. 'cd then touch then ls'), execute each step in order.\n\
                 If the user asks for only a single operation, execute just that one operation.\n\
                 Tool call format (exact):\n\
                 TOOL: tool_name\n\
                 ARGS: {json}\n\
                 After each tool call, the system provides the Observation. Continue calling tools until the entire task is complete.\n\
                 When all requested operations are finished, respond with DONE and no more tool calls.\n\
                 Do not provide conversational commentary or summaries."
            }
            AiCommand::Plan => {
                "You are a planning assistant. Produce a clear, numbered step-by-step plan for the goal.\n\
                 Use read-only tools (ls, cat, grep, web_search) only if they help discover facts for the plan.\n\
                 Do not perform destructive actions. At the end, output the plan clearly."
            }
            AiCommand::Build => {
                "You are an autonomous agentic software builder (similar to Grok Build).\n\
                 Your job is to build, code, and create complete working projects, features, and files.\n\
                 Follow this workflow:\n\
                 1. If needed, explore existing files with ls, cat, or grep.\n\
                 2. Write complete, high-quality, production-ready code using write_file. Do not truncate code or leave placeholders.\n\
                 3. Create directories with mkdir if needed.\n\
                 4. You can execute shell commands or tests with exec_cmd.\n\
                 Tool call format (must be valid JSON):\n\
                 TOOL: tool_name\n\
                 ARGS: {\"path\": \"...\", \"content\": \"...\"}\n\
                 After each tool call, you will receive the Observation. Continue calling tools until all files and tasks are fully built.\n\
                 When finished building, output a short summary of created files and what was built."
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentUpdate {
    Thinking {
        step: usize,
        max_steps: usize,
        thought: String,
    },
    ToolStarted {
        step: usize,
        tool: String,
        command_or_args: String,
    },
    ToolOutput {
        tool: String,
        lines: Vec<String>,
    },
    ToolFinished {
        tool: String,
        summary: String,
    },
    Completed {
        final_answer: Option<String>,
        tool_output_lines: Vec<String>,
    },
    Error(String),
}

/// Extract all @file or @folder tokens from query (including quoted e.g. @"foo/bar")
fn extract_at_references(query: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let chars: Vec<char> = query.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '@' {
            i += 1;
            if i >= chars.len() {
                break;
            }
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                let path: String = chars[start..i].iter().collect();
                if !path.trim().is_empty() {
                    refs.push(path.trim().to_string());
                }
                if i < chars.len() {
                    i += 1;
                }
            } else {
                let start = i;
                while i < chars.len()
                    && !chars[i].is_whitespace()
                    && chars[i] != ','
                    && chars[i] != ';'
                    && chars[i] != ')'
                    && chars[i] != ']'
                {
                    i += 1;
                }
                let raw: String = chars[start..i].iter().collect();
                let clean = raw.trim_matches(|c: char| {
                    !c.is_alphanumeric() && c != '.' && c != '/' && c != '_' && c != '-'
                });
                if !clean.is_empty() {
                    refs.push(clean.to_string());
                }
            }
        } else {
            i += 1;
        }
    }
    refs
}

/// If a file does not exist directly at the root, search subdirectories for a matching file name
fn find_file_in_subdirs(cwd: &std::path::Path, target_name: &str) -> Option<std::path::PathBuf> {
    if target_name.contains('/') {
        return None;
    }
    let mut found = None;
    fn walk(
        dir: &std::path::Path,
        target: &str,
        depth: usize,
        found: &mut Option<std::path::PathBuf>,
    ) {
        if depth > 4 || found.is_some() {
            return;
        }
        let rd = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => return,
        };
        for entry in rd.flatten() {
            if found.is_some() {
                break;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            let path = entry.path();
            if path.is_file() && name == target {
                *found = Some(path);
                return;
            } else if path.is_dir() {
                walk(&path, target, depth + 1, found);
            }
        }
    }
    walk(cwd, target_name, 0, &mut found);
    found
}

/// Resolve @filepath and @folder references in the user query against the working directory
fn resolve_file_references(query: &str, cwd: &str) -> (String, Vec<String>) {
    let mut extra_context = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();
    let cwd_path = std::path::Path::new(cwd);

    let extracted = extract_at_references(query);
    for target in extracted {
        if !seen_paths.insert(target.clone()) {
            continue;
        }

        let full = cwd_path.join(&target);
        if full.is_file() {
            if let Ok(content) = std::fs::read_to_string(&full) {
                let snippet = if content.len() > 30_000 {
                    format!("{}... [truncated]", &content[..30_000])
                } else {
                    content
                };
                extra_context.push(format!(
                    "Referenced file @{}:\n```\n{}\n```",
                    target, snippet
                ));
            }
        } else if full.is_dir() {
            let folder_ctx = format_folder_context(&full, &target);
            extra_context.push(folder_ctx);
        } else if let Some(sub_path) = find_file_in_subdirs(cwd_path, &target) {
            // Found in subdirectories (e.g. @agent.rs -> src/ai/agent.rs)
            if let Ok(content) = std::fs::read_to_string(&sub_path) {
                let rel = sub_path
                    .strip_prefix(cwd_path)
                    .unwrap_or(&sub_path)
                    .to_string_lossy();
                let snippet = if content.len() > 30_000 {
                    format!("{}... [truncated]", &content[..30_000])
                } else {
                    content
                };
                extra_context.push(format!(
                    "Referenced file @{} (located at {}):\n```\n{}\n```",
                    target, rel, snippet
                ));
            }
        }
    }

    (query.to_string(), extra_context)
}

fn format_folder_context(dir_path: &std::path::Path, rel_label: &str) -> String {
    let mut out = format!("Referenced folder @{}:\nDirectory structure:\n", rel_label);
    let mut files_to_read = Vec::new();
    let mut tree_lines = Vec::new();
    let mut count = 0;

    fn walk_dir(
        dir: &std::path::Path,
        current_depth: usize,
        max_depth: usize,
        lines: &mut Vec<String>,
        files: &mut Vec<std::path::PathBuf>,
        count: &mut usize,
    ) {
        if current_depth > max_depth || *count >= 60 {
            return;
        }
        let mut entries: Vec<_> = match std::fs::read_dir(dir) {
            Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
            Err(_) => return,
        };
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            if *count >= 60 {
                break;
            }
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                continue;
            }
            *count += 1;
            let indent = "  ".repeat(current_depth + 1);
            let path = entry.path();
            if path.is_dir() {
                lines.push(format!("{}{}/", indent, file_name));
                walk_dir(&path, current_depth + 1, max_depth, lines, files, count);
            } else {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                lines.push(format!("{}{}{}", indent, file_name, format_size_suffix(size)));
                if files.len() < 5 && is_inspectable_file(&file_name) && size < 25_000 {
                    files.push(path);
                }
            }
        }
    }

    walk_dir(dir_path, 0, 3, &mut tree_lines, &mut files_to_read, &mut count);
    out.push_str(&tree_lines.join("\n"));

    if !files_to_read.is_empty() {
        out.push_str("\n\nKey file contents:\n");
        let mut total_bytes = 0;
        for file in files_to_read {
            if total_bytes > 30_000 {
                break;
            }
            if let Ok(content) = std::fs::read_to_string(&file) {
                let name = file.file_name().unwrap_or_default().to_string_lossy();
                let snippet = if content.len() > 6000 {
                    format!("{}... [truncated]", &content[..6000])
                } else {
                    content
                };
                total_bytes += snippet.len();
                out.push_str(&format!("\n--- {} ---\n{}\n", name, snippet));
            }
        }
    }

    out
}

fn format_size_suffix(bytes: u64) -> String {
    if bytes < 1024 {
        format!(" ({} B)", bytes)
    } else if bytes < 1024 * 1024 {
        format!(" ({:.1} KB)", bytes as f64 / 1024.0)
    } else {
        format!(" ({:.1} MB)", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn is_inspectable_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.split('.').last(),
        Some("rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "html" | "css" | "json" | "toml" | "yaml" | "yml" | "md" | "sh" | "sql")
    )
}

fn extract_reasoning_thought(response: &str) -> Option<String> {
    if let Some(start) = response.find("<think>") {
        if let Some(end) = response[start + 7..].find("</think>") {
            let thought = response[start + 7..start + 7 + end].trim();
            if !thought.is_empty() {
                return Some(thought.to_string());
            }
        }
    }

    let lower = response.to_lowercase();
    let cut_pos = [
        lower.find("tool:"),
        lower.find("action:"),
        lower.find("```json"),
        lower.find("```"),
        lower.find("{\"tool\""),
        lower.find("{\"name\""),
    ]
    .into_iter()
    .flatten()
    .min();

    if let Some(pos) = cut_pos {
        let before = response[..pos].trim();
        if !before.is_empty() && before.len() > 3 {
            return Some(before.to_string());
        }
    }

    None
}

fn format_tool_args_summary(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "grep" => {
            let pat = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            format!("pattern: \"{}\" in {}", pat, path)
        }
        "exec_cmd" | "run_command" | "exec" | "bash" | "sh" => {
            args.get("cmd").and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
        "cat" => {
            args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
        "ls" | "dir" => {
            args.get("path").and_then(|v| v.as_str()).unwrap_or(".").to_string()
        }
        "write_file" | "write" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let len = args.get("content").and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0);
            format!("{} ({} bytes)", path, len)
        }
        "mkdir" | "touch" | "delete_path" | "delete" | "rm" => {
            args.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
        "copy_path" | "move_path" => {
            let src = args.get("src").and_then(|v| v.as_str()).unwrap_or("");
            let dst = args.get("dst").and_then(|v| v.as_str()).unwrap_or("");
            format!("{} -> {}", src, dst)
        }
        "web_search" => {
            args.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
        _ => args.to_string(),
    }
}

fn format_tool_summary(tool_name: &str, tool_res: &serde_json::Value, args_summary: &str) -> String {
    if let Some(err) = tool_res.get("error").and_then(|v| v.as_str()) {
        return format!("[{}] Error: {}", tool_name, err);
    }
    match tool_name {
        "grep" => {
            let count = tool_res.get("results").and_then(|v| v.as_str()).map(|s| s.lines().count()).unwrap_or(0);
            format!("[grep] Found {} matching lines ({})", count, args_summary)
        }
        "write_file" | "write" => {
            if let Some(path) = tool_res.get("path").and_then(|v| v.as_str()) {
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                format!("[write_file] Created {} ({} bytes)", path, size)
            } else {
                format!("[write_file] {}", args_summary)
            }
        }
        "mkdir" => {
            let p = tool_res.get("path").and_then(|v| v.as_str()).unwrap_or(args_summary);
            format!("[mkdir] Created directory {}", p)
        }
        "touch" => {
            let p = tool_res.get("path").and_then(|v| v.as_str()).unwrap_or(args_summary);
            format!("[touch] Touched {}", p)
        }
        "delete_path" | "delete" | "rm" => {
            let p = tool_res.get("path").and_then(|v| v.as_str()).unwrap_or(args_summary);
            format!("[delete] Deleted {}", p)
        }
        "exec_cmd" | "run_command" | "exec" | "bash" | "sh" => {
            format!("[exec_cmd] Finished: {}", args_summary)
        }
        "copy_path" => format!("[copy] {}", args_summary),
        "move_path" => format!("[move] {}", args_summary),
        "web_search" => {
            let count = tool_res.as_array().map(|a| a.len()).unwrap_or(0);
            format!("[web_search] Found {} results for \"{}\"", count, args_summary)
        }
        _ => String::new(),
    }
}

/// Main entry for running an AI command. Returns lines to display as output.
pub async fn run_ai_command(
    cmd: AiCommand,
    user_query: &str,
    ai_config: AiConfig,
    storage: &LocalStorage,
    cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    update_tx: Option<std::sync::mpsc::Sender<AgentUpdate>>,
) -> Result<Vec<String>, AiError> {
    if !ai_config.enabled {
        return Err(AiError::NotEnabled);
    }

    let provider: AiProvider = create_provider(ai_config.clone())?;

    // Try to init RAG (best effort; failures are non-fatal)
    let rag = RagEngine::new_from_config(storage, &ai_config, None).await.ok();
    let rag_context = if let Some(r) = &rag {
        r.retrieve_context(user_query, 3).await
    } else {
        String::new()
    };

    let cwd = env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "~".into());

    let (clean_query, ref_contexts) = resolve_file_references(user_query, &cwd);

    let tool_defs = get_tool_definitions();
    let tools_json = serde_json::to_string(&tool_defs).unwrap_or_default();

    let mut history: Vec<String> = vec![
        format!("System: {}", cmd.system_prompt()),
        format!("Tools available:\n{}", tools_json),
    ];
    if !rag_context.is_empty() {
        history.push(rag_context);
    }
    for ctx in ref_contexts {
        history.push(ctx);
    }
    history.push(format!("Context: cwd={}", cwd));
    history.push(format!("User: {}", clean_query));

    let max_steps = cmd.max_steps();
    let mut final_answer: Option<String> = None;
    let mut tool_output_lines: Vec<String> = Vec::new();
    let mut tools_executed = 0;

    for _step in 0..max_steps {
        if let Some(ref cancel) = cancel_token {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(vec!["^C [Process stopped by Esc]".to_string()]);
            }
        }

        let prompt = history.join("\n\n");

        let response = match provider.chat(vec![prompt]).await {
            Ok(r) => r,
            Err(e) => {
                if let Some(ref tx) = update_tx {
                    let _ = tx.send(AgentUpdate::Error(e.to_string()));
                }
                return Err(e);
            }
        };

        if let Some(ref cancel) = cancel_token {
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(vec!["^C [Process stopped by Esc]".to_string()]);
            }
        }

        // Check if there is reasoning/thought to stream to TUI
        if let Some(thought) = extract_reasoning_thought(&response) {
            if let Some(ref tx) = update_tx {
                let _ = tx.send(AgentUpdate::Thinking {
                    step: _step,
                    max_steps,
                    thought,
                });
            }
        }

        // Parse tool calls (supports one or multiple TOOL: blocks per response)
        let tool_calls = parse_tool_calls(&response);
        if !tool_calls.is_empty() {
            history.push(format!("Assistant: {}", response.trim()));

            for (tool_name, args_json) in tool_calls {
                if let Some(ref cancel) = cancel_token {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        return Ok(vec!["^C [Process stopped by Esc]".to_string()]);
                    }
                }

                let args_summary = format_tool_args_summary(&tool_name, &args_json);
                if let Some(ref tx) = update_tx {
                    let _ = tx.send(AgentUpdate::ToolStarted {
                        step: _step,
                        tool: tool_name.clone(),
                        command_or_args: args_summary.clone(),
                    });
                }

                let tool_res = execute_tool(&tool_name, args_json).await
                    .unwrap_or_else(|e| serde_json::json!({"error": e}));

                tools_executed += 1;
                let lines = format_tool_output_for_terminal(&tool_name, &tool_res);
                if (cmd == AiCommand::Do || cmd == AiCommand::Build) && !lines.is_empty() {
                    tool_output_lines.extend(lines.clone());
                }

                let summary = format_tool_summary(&tool_name, &tool_res, &args_summary);
                if let Some(ref tx) = update_tx {
                    if !lines.is_empty() {
                        let _ = tx.send(AgentUpdate::ToolOutput {
                            tool: tool_name.clone(),
                            lines,
                        });
                    }
                    if !summary.is_empty() {
                        let _ = tx.send(AgentUpdate::ToolFinished {
                            tool: tool_name.clone(),
                            summary,
                        });
                    }
                }

                let obs = format!("Observation (tool={}): {}", tool_name, tool_res);
                history.push(obs);
            }

            // Continue to next turn to let model inspect observations or do next steps
            continue;
        } else {
            // Final answer
            final_answer = Some(response.trim().to_string());
            break;
        }
    }

    let mut result = Vec::new();
    if (cmd == AiCommand::Do || cmd == AiCommand::Build) && tools_executed > 0 {
        result.extend(tool_output_lines.clone());
        if let Some(ref ans) = final_answer {
            let clean = ans.trim();
            let lower = clean.to_lowercase();
            if !clean.is_empty()
                && lower != "done"
                && lower != "done."
                && !lower.starts_with("done")
                && !clean.starts_with("TOOL:")
            {
                if !result.is_empty() {
                    result.push(String::new());
                }
                result.extend(clean.lines().map(|s| s.to_string()));
            }
        }
    } else if let Some(ref ans) = final_answer {
        result.extend(ans.lines().map(|s| s.to_string()));
    }

    if let Some(ref tx) = update_tx {
        let _ = tx.send(AgentUpdate::Completed {
            final_answer: final_answer.clone(),
            tool_output_lines: tool_output_lines.clone(),
        });
    }

    Ok(result)
}

fn format_tool_output_for_terminal(tool_name: &str, tool_res: &serde_json::Value) -> Vec<String> {
    if let Some(err) = tool_res.get("error").and_then(|v| v.as_str()) {
        return vec![format!("{}: {}", tool_name, err)];
    }

    match tool_name {
        "cat" => {
            if let Some(content) = tool_res.get("content").and_then(|v| v.as_str()) {
                return content.lines().map(|s| s.to_string()).collect();
            }
        }
        "ls" | "dir" => {
            if let Some(arr) = tool_res.as_array() {
                let mut lines = Vec::new();
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        let is_dir = item.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                        if is_dir {
                            lines.push(format!("{}/", name));
                        } else {
                            lines.push(name.to_string());
                        }
                    }
                }
                return lines;
            }
        }
        "grep" => {
            if let Some(results) = tool_res.get("results").and_then(|v| v.as_str()) {
                if !results.is_empty() {
                    return results.lines().map(|s| s.to_string()).collect();
                }
            }
        }
        "exec_cmd" | "run_command" | "exec" | "bash" | "sh" => {
            if let Some(out) = tool_res.get("output").and_then(|v| v.as_str()) {
                if !out.is_empty() {
                    return out.lines().map(|s| s.to_string()).collect();
                }
            }
        }
        "web_search" => {
            if let Some(arr) = tool_res.as_array() {
                let mut lines = Vec::new();
                for item in arr {
                    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
                    if !title.is_empty() || !snippet.is_empty() {
                        lines.push(format!("• {} ({})", title, url));
                        if !snippet.is_empty() {
                            lines.push(format!("  {}", snippet));
                        }
                    }
                }
                return lines;
            }
        }
        "write_file" | "write" => {
            if let Some(path) = tool_res.get("path").and_then(|v| v.as_str()) {
                let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                if size > 0 {
                    return vec![format!("[write_file] Created {} ({} bytes)", path, size)];
                }
                return vec![format!("[write_file] Written to {}", path)];
            }
        }
        "mkdir" => {
            if let Some(path) = tool_res.get("path").and_then(|v| v.as_str()) {
                return vec![format!("[mkdir] Created directory {}", path)];
            }
        }
        "touch" => {
            if let Some(path) = tool_res.get("path").and_then(|v| v.as_str()) {
                return vec![format!("[touch] Touched {}", path)];
            }
        }
        "delete_path" | "delete" | "rm" => {
            if let Some(path) = tool_res.get("path").and_then(|v| v.as_str()) {
                return vec![format!("[delete] Deleted {}", path)];
            }
        }
        "copy_path" | "copy" | "cp" => {
            let src = tool_res.get("src").and_then(|v| v.as_str()).unwrap_or("");
            let dst = tool_res.get("dst").and_then(|v| v.as_str()).unwrap_or("");
            return vec![format!("[copy] Copied {} -> {}", src, dst)];
        }
        "move_path" | "move" | "mv" => {
            let src = tool_res.get("src").and_then(|v| v.as_str()).unwrap_or("");
            let dst = tool_res.get("dst").and_then(|v| v.as_str()).unwrap_or("");
            return vec![format!("[move] Moved {} -> {}", src, dst)];
        }
        "cd" => {
            if let Some(new_cwd) = tool_res.get("cwd").and_then(|v| v.as_str()) {
                return vec![format!("[cd] Changed directory to {}", new_cwd)];
            }
        }
        _ => {}
    }

    Vec::new()
}

/// Extract balanced JSON value from text, handling trailing commentary,
/// unescaped control chars in string literals, and markdown fences.
pub fn extract_balanced_json(text: &str) -> Option<serde_json::Value> {
    extract_balanced_json_with_len(text).map(|(v, _)| v)
}

/// Extract balanced JSON value and number of bytes consumed from text.
pub fn extract_balanced_json_with_len(text: &str) -> Option<(serde_json::Value, usize)> {
    let mut s = text.trim_start();
    let prefix_offset = text.len() - s.len();
    let mut fence_offset = 0;
    if let Some(idx) = s.find("```json") {
        fence_offset = idx + 7;
        s = &s[fence_offset..];
    } else if let Some(idx) = s.find("```") {
        fence_offset = idx + 3;
        s = &s[fence_offset..];
    }

    let start_pos = s.find(|c| c == '{' || c == '[')?;
    let start_char = s[start_pos..].chars().next().unwrap();
    let is_object = start_char == '{';
    let target_close = if is_object { '}' } else { ']' };

    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut end_pos = None;

    for (byte_idx, ch) in s[start_pos..].char_indices() {
        let abs_idx = start_pos + byte_idx;
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if (is_object && ch == '{') || (!is_object && ch == '[') {
                depth += 1;
            } else if ch == target_close {
                depth -= 1;
                if depth == 0 {
                    end_pos = Some(abs_idx + ch.len_utf8());
                    break;
                }
            }
        }
    }

    if let Some(end) = end_pos {
        let candidate = &s[start_pos..end];
        let total_consumed = prefix_offset + fence_offset + end;
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
            return Some((v, total_consumed));
        }
        let sanitized = sanitize_json_string(candidate);
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&sanitized) {
            return Some((v, total_consumed));
        }
    }

    // Truncation fallback: attempt auto-closing
    let candidate = &s[start_pos..];
    let total_consumed = text.len();
    let mut auto_closed = candidate.to_string();
    if in_string {
        auto_closed.push('"');
    }
    for _ in 0..depth {
        auto_closed.push(target_close);
    }
    let sanitized = sanitize_json_string(&auto_closed);
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&sanitized) {
        return Some((v, total_consumed));
    }

    None
}

/// Replace literal unescaped control characters inside JSON strings.
fn sanitize_json_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    let mut in_string = false;
    let mut escape = false;

    for ch in raw.chars() {
        if escape {
            out.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            out.push(ch);
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            out.push(ch);
            continue;
        }
        if in_string {
            match ch {
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c => out.push(c),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Parse one or multiple tool calls from model output.
pub fn parse_tool_calls(text: &str) -> Vec<(String, serde_json::Value)> {
    let mut calls = Vec::new();
    let t = text.trim();

    // 1) Case-insensitive search for "TOOL:"
    let t_lower = t.to_ascii_lowercase();
    let mut search_idx = 0;
    while search_idx < t_lower.len() {
        if let Some(rel_tool_idx) = t_lower[search_idx..].find("tool:") {
            let tool_start = search_idx + rel_tool_idx;
            let after_tool = &t[tool_start + 5..];
            let tool_line = after_tool.lines().next().unwrap_or("").trim();
            let tool_name = tool_line
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                .to_lowercase();

            let after_tool_lower = &t_lower[tool_start + 5..];
            if let Some(rel_args_idx) = after_tool_lower.find("args:") {
                let args_start = tool_start + 5 + rel_args_idx + 5;
                if args_start <= t.len() {
                    let args_sub = &t[args_start..];
                    if let Some((val, consumed)) = extract_balanced_json_with_len(args_sub) {
                        calls.push((tool_name, val));
                        search_idx = std::cmp::min(args_start + consumed, t_lower.len());
                        continue;
                    }
                }
                search_idx = std::cmp::min(tool_start + 5, t_lower.len());
            } else if let Some((val, consumed)) = extract_balanced_json_with_len(after_tool) {
                calls.push((tool_name, val));
                search_idx = std::cmp::min(tool_start + 5 + consumed, t_lower.len());
            } else {
                search_idx = std::cmp::min(tool_start + 5, t_lower.len());
            }
        } else {
            break;
        }
    }

    if !calls.is_empty() {
        return calls;
    }

    // 2) Look for "Action: <name>\nAction Input: <json>"
    if let Some(act_idx) = t_lower.find("action:") {
        let after_act = &t[act_idx + 7..];
        let tool_name = after_act
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .to_lowercase();
        let after_act_lower = &t_lower[act_idx + 7..];
        if let Some(input_idx) = after_act_lower.find("action input:") {
            let args_sub = &after_act[input_idx + 13..];
            if let Some(val) = extract_balanced_json(args_sub) {
                return vec![(tool_name, val)];
            }
        }
    }

    // 3) Fallback: JSON blob like {"name": "...", "arguments": {...}} or {"tool": "...", "args": {...}}
    if let Some(val) = extract_balanced_json(t) {
        if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
            let args = val.get("arguments").cloned().unwrap_or(serde_json::json!({}));
            return vec![(name.to_string(), args)];
        }
        if let Some(name) = val.get("tool").and_then(|v| v.as_str()) {
            let args = val.get("args").cloned().unwrap_or(serde_json::json!({}));
            return vec![(name.to_string(), args)];
        }
    }

    calls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls_single() {
        let text = "TOOL: ls\nARGS: {\"path\": \".\"}";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "ls");
        assert_eq!(calls[0].1["path"], ".");
    }

    #[test]
    fn test_parse_tool_calls_multiple() {
        let text = "TOOL: cd\nARGS: {\"path\": \"src\"}\n\nTOOL: touch\nARGS: {\"path\": \"new.md\"}\n\nTOOL: ls\nARGS: {}";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, "cd");
        assert_eq!(calls[0].1["path"], "src");
        assert_eq!(calls[1].0, "touch");
        assert_eq!(calls[1].1["path"], "new.md");
        assert_eq!(calls[2].0, "ls");
    }

    #[test]
    fn test_parse_tool_calls_multiline_json() {
        let text = "TOOL: write_file\nARGS: {\n  \"path\": \"test.txt\",\n  \"content\": \"hello world\"\n}";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "write_file");
        assert_eq!(calls[0].1["path"], "test.txt");
        assert_eq!(calls[0].1["content"], "hello world");
    }

    #[test]
    fn test_parse_tool_calls_no_tools() {
        let text = "I am done. All files have been created.";
        let calls = parse_tool_calls(text);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_tool_calls_json_object_fallback() {
        let text = "```json\n{\"tool\": \"cat\", \"args\": {\"path\": \"README.md\"}}\n```";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "cat");
        assert_eq!(calls[0].1["path"], "README.md");
    }

    #[test]
    fn test_parse_tool_calls_with_trailing_explanation() {
        let text = "TOOL: write_file\nARGS: {\"path\": \"index.html\", \"content\": \"<h1>Hello</h1>\"}\n\nI have created the landing page for you.";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "write_file");
        assert_eq!(calls[0].1["path"], "index.html");
        assert_eq!(calls[0].1["content"], "<h1>Hello</h1>");
    }

    #[test]
    fn test_parse_tool_calls_with_literal_newlines_in_json() {
        let text = "TOOL: write_file\nARGS: {\"path\": \"index.html\", \"content\": \"<!DOCTYPE html>\n<html>\n<body>\n<h1>Title</h1>\n</body>\n</html>\"}";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "write_file");
        assert_eq!(calls[0].1["path"], "index.html");
        assert!(calls[0].1["content"].as_str().unwrap().contains("<h1>Title</h1>"));
    }

    #[test]
    fn test_parse_tool_calls_action_input_format() {
        let text = "Action: write_file\nAction Input: {\"path\": \"app.py\", \"content\": \"print('hi')\"}";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "write_file");
        assert_eq!(calls[0].1["path"], "app.py");
    }

    #[test]
    fn test_resolve_file_references_nonexistent() {
        let (q, extra) = resolve_file_references("make page from @nonexistent.html", ".");
        assert_eq!(q, "make page from @nonexistent.html");
        assert!(extra.is_empty());
    }

    #[test]
    fn test_resolve_file_references_existing() {
        let temp = std::env::temp_dir().join(format!("nsh_test_file_ref_{}.txt", std::process::id()));
        let _ = std::fs::write(&temp, "file content here");
        let query = format!("look at @{}", temp.file_name().unwrap().to_string_lossy());
        let (_q, extra) = resolve_file_references(&query, temp.parent().unwrap().to_str().unwrap());
        assert_eq!(extra.len(), 1);
        assert!(extra[0].contains("file content here"));
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn test_resolve_folder_references_existing() {
        let temp = std::env::temp_dir().join(format!("nsh_test_dir_ref_{}", std::process::id()));
        let _ = std::fs::create_dir_all(temp.join("subdir"));
        let _ = std::fs::write(temp.join("hello.rs"), "fn hello() {}");
        let query = format!("look at @{}", temp.file_name().unwrap().to_string_lossy());
        let (_q, extra) = resolve_file_references(&query, temp.parent().unwrap().to_str().unwrap());
        assert_eq!(extra.len(), 1);
        assert!(extra[0].contains("Referenced folder @"));
        assert!(extra[0].contains("hello.rs"));
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn test_resolve_file_references_nested_fallback() {
        let temp = std::env::temp_dir().join(format!("nsh_test_nested_{}", std::process::id()));
        let nested_dir = temp.join("sub").join("nested");
        let _ = std::fs::create_dir_all(&nested_dir);
        let target_file = nested_dir.join("deep_code.rs");
        let _ = std::fs::write(&target_file, "pub fn deep() -> bool { true }");

        // Query mentions @deep_code.rs without full path
        let query = "look at @deep_code.rs please";
        let (_q, extra) = resolve_file_references(query, temp.to_str().unwrap());
        assert_eq!(extra.len(), 1);
        assert!(extra[0].contains("pub fn deep() -> bool { true }"));
        assert!(extra[0].contains("deep_code.rs"));

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn test_resolve_file_references_quoted() {
        let temp = std::env::temp_dir().join(format!("nsh_test_quoted_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp);
        let file_path = temp.join("my spaced file.txt");
        let _ = std::fs::write(&file_path, "spaced content");

        let query = "check @\"my spaced file.txt\" here";
        let (_q, extra) = resolve_file_references(query, temp.to_str().unwrap());
        assert_eq!(extra.len(), 1);
        assert!(extra[0].contains("spaced content"));

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn test_resolve_multiple_folders() {
        let temp = std::env::temp_dir().join(format!("nsh_test_multi_{}", std::process::id()));
        let dir1 = temp.join("folder_a");
        let dir2 = temp.join("folder_b");
        let _ = std::fs::create_dir_all(&dir1);
        let _ = std::fs::create_dir_all(&dir2);
        let _ = std::fs::write(dir1.join("a.rs"), "// file a");
        let _ = std::fs::write(dir2.join("b.rs"), "// file b");

        let query = "build feature based on @folder_a and @folder_b";
        let (_q, extra) = resolve_file_references(query, temp.to_str().unwrap());
        assert_eq!(extra.len(), 2);
        assert!(extra.iter().any(|s| s.contains("folder_a") && s.contains("a.rs")));
        assert!(extra.iter().any(|s| s.contains("folder_b") && s.contains("b.rs")));

        let _ = std::fs::remove_dir_all(temp);
    }
}



