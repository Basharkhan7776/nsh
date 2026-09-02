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
                "You are an agentic builder. Explore the workspace with ls/grep/cat, write code, create directories and files using tools.\n\
                 Work iteratively: explore first, then create/edit files with write_file.\n\
                 Use this exact format for tool use:\n\
                 TOOL: name\n\
                 ARGS: {\"path\": \"...\", \"content\": \"...\"}\n\
                 When finished, output a short summary + list of created/modified paths."
            }
        }
    }
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

    let tool_defs = get_tool_definitions();
    let tools_json = serde_json::to_string(&tool_defs).unwrap_or_default();

    let mut history: Vec<String> = vec![
        format!("System: {}", cmd.system_prompt()),
        format!("Tools available:\n{}", tools_json),
    ];
    if !rag_context.is_empty() {
        history.push(rag_context);
    }
    history.push(format!("Context: cwd={}", cwd));
    history.push(format!("User: {}", user_query));

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
                if cmd == AiCommand::Do {
                    let lines = format_tool_output_for_terminal(&tool_name, &tool_res);
                    tool_output_lines.extend(lines);
                }

                let obs = format!("Observation (tool={}): {}", tool_name, tool_res);
                history.push(obs);
            }

            // For plan we rarely want to keep mutating after first tool, but continue
            continue;
        } else {
            // Final answer
            final_answer = Some(response.trim().to_string());
            break;
        }
    }

    if cmd == AiCommand::Do && tools_executed > 0 {
        if tool_output_lines.is_empty() {
            if let Some(ref ans) = final_answer {
                let clean = ans.trim();
                let lower = clean.to_lowercase();
                if !clean.is_empty()
                    && lower != "done"
                    && lower != "done."
                    && !lower.starts_with("done")
                {
                    return Ok(clean.lines().map(|s| s.to_string()).collect());
                }
            }
        }
        return Ok(tool_output_lines);
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
        "cd" | "touch" | "write_file" | "write" | "delete_path" | "delete" | "rm"
        | "copy_path" | "copy" | "cp" | "move_path" | "move" | "mv" | "mkdir" => {
            return Vec::new();
        }
        _ => {}
    }

    Vec::new()
}

/// Parse one or multiple tool calls from model output.
pub fn parse_tool_calls(text: &str) -> Vec<(String, serde_json::Value)> {
    let mut calls = Vec::new();
    let t = text.trim();

    // 1) Look for all "TOOL:" occurrences
    let mut search_idx = 0;
    while let Some(rel_tool_idx) = t[search_idx..].find("TOOL:") {
        let tool_start = search_idx + rel_tool_idx;
        let after_tool = &t[tool_start + 5..];
        let tool_line = after_tool.lines().next().unwrap_or("").trim();
        let tool_name = tool_line.to_string();

        if let Some(rel_args_idx) = after_tool.find("ARGS:") {
            let args_start = tool_start + 5 + rel_args_idx + 5;
            // End of this tool call is the next "TOOL:" or end of text
            let args_end = match t[args_start..].find("TOOL:") {
                Some(next_tool_rel) => args_start + next_tool_rel,
                None => t.len(),
            };
            let raw_args_str = t[args_start..args_end].trim();
            let json_str = clean_json_str(raw_args_str);

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                calls.push((tool_name, val));
            } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw_args_str.lines().next().unwrap_or("")) {
                calls.push((tool_name, val));
            }
            search_idx = args_end;
        } else {
            search_idx = tool_start + 5;
        }
    }

    if !calls.is_empty() {
        return calls;
    }

    // 2) Fallback: JSON blob like {"name": "...", "arguments": {...}} or {"tool": "...", "args": {...}}
    if let Some(start) = t.find('{') {
        if let Some(end) = t[start..].rfind('}') {
            let candidate = &t[start..start + end + 1];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(candidate) {
                if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                    let args = val.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                    return vec![(name.to_string(), args)];
                }
                if let Some(name) = val.get("tool").and_then(|v| v.as_str()) {
                    let args = val.get("args").cloned().unwrap_or(serde_json::json!({}));
                    return vec![(name.to_string(), args)];
                }
            }
        }
    }

    calls
}

fn clean_json_str(raw: &str) -> String {
    let mut s = raw.trim();
    if s.starts_with("```json") {
        s = &s[7..];
    } else if s.starts_with("```") {
        s = &s[3..];
    }
    if let Some(idx) = s.rfind("```") {
        s = &s[..idx];
    }
    s.trim().to_string()
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
}
