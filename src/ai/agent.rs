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

    fn max_steps(&self) -> usize {
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

/// Resolve @filepath references in the user query against the working directory
fn resolve_file_references(query: &str, cwd: &str) -> (String, Vec<String>) {
    let mut extra_context = Vec::new();
    for word in query.split_whitespace() {
        if let Some(target) = word.strip_prefix('@') {
            let clean_path = target.trim_matches(|c: char| {
                !c.is_alphanumeric() && c != '.' && c != '/' && c != '_' && c != '-'
            });
            if !clean_path.is_empty() {
                let full = std::path::Path::new(cwd).join(clean_path);
                if full.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&full) {
                        let snippet = if content.len() > 30_000 {
                            format!("{}... [truncated]", &content[..30_000])
                        } else {
                            content
                        };
                        extra_context.push(format!(
                            "Referenced file @{}:\n```\n{}\n```",
                            clean_path, snippet
                        ));
                    }
                }
            }
        }
    }
    (query.to_string(), extra_context)
}

/// Main entry for running an AI command. Returns lines to display as output.
pub async fn run_ai_command(
    cmd: AiCommand,
    user_query: &str,
    ai_config: AiConfig,
    storage: &LocalStorage,
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
        let prompt = history.join("\n\n");

        let response = provider.chat(vec![prompt]).await?;

        // Parse tool calls (supports one or multiple TOOL: blocks per response)
        let tool_calls = parse_tool_calls(&response);
        if !tool_calls.is_empty() {
            history.push(format!("Assistant: {}", response.trim()));

            for (tool_name, args_json) in tool_calls {
                let tool_res = execute_tool(&tool_name, args_json).await
                    .unwrap_or_else(|e| serde_json::json!({"error": e}));

                tools_executed += 1;
                if cmd == AiCommand::Do || cmd == AiCommand::Build {
                    let lines = format_tool_output_for_terminal(&tool_name, &tool_res);
                    tool_output_lines.extend(lines);
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

    if (cmd == AiCommand::Do || cmd == AiCommand::Build) && tools_executed > 0 {
        let mut result = tool_output_lines;
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
        if !result.is_empty() {
            return Ok(result);
        }
    }

    let answer = final_answer.unwrap_or_else(|| {
        "The agent finished without a clear final message. (See history above if shown.)".to_string()
    });

    // Return as display lines (split)
    Ok(answer.lines().map(|s| s.to_string()).collect())
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
}

