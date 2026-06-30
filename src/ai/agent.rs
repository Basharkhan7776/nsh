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
            AiCommand::Do => 3,
            AiCommand::Plan => 2,
            AiCommand::Build => 6,
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
                "You are an agent that performs small terminal tasks (copy, move, delete, write small files, mkdir).\n\
                 Use the available tools. Prefer precise single-file operations.\n\
                 Tool call format (exact):\n\
                 TOOL: tool_name\n\
                 ARGS: {json}\n\
                 After tool results come back as Observation, continue until task complete.\n\
                 Finally output a short summary of what you did."
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

    let provider: AiProvider = create_provider(ai_config.clone());

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

    for _step in 0..max_steps {
        let prompt = history.join("\n\n");

        let response = provider.chat(vec![prompt]).await?;

        // Simple parser: look for TOOL: and ARGS:
        if let Some((tool_name, args_json)) = parse_tool_call(&response) {
            history.push(format!("Assistant: {}", response.trim()));

            // Execute
            let tool_res = execute_tool(&tool_name, args_json).await
                .unwrap_or_else(|e| serde_json::json!({"error": e}));

            let obs = format!("Observation (tool={}): {}", tool_name, tool_res);
            history.push(obs);

            // For plan we rarely want to keep mutating after first tool, but continue
            continue;
        } else {
            // Final answer
            final_answer = Some(response.trim().to_string());
            break;
        }
    }

    let answer = final_answer.unwrap_or_else(|| {
        "The agent finished without a clear final message. (See history above if shown.)".to_string()
    });

    // Return as display lines (split)
    Ok(answer.lines().map(|s| s.to_string()).collect())
}

fn parse_tool_call(text: &str) -> Option<(String, serde_json::Value)> {
    let t = text.trim();
    // Accept a few common shapes
    // 1) TOOL: foo\nARGS: {..}
    if let Some(tool_start) = t.find("TOOL:") {
        let after = &t[tool_start + 5..];
        let tool_line = after.lines().next().unwrap_or("").trim();
        let tool = tool_line.to_string();

        if let Some(args_start) = t.find("ARGS:") {
            let rest = &t[args_start + 5..];
            let json_part = rest.trim_start();
            // take until end of line or ``` or next keyword
            let json_str = json_part
                .lines()
                .take(1)
                .collect::<Vec<_>>()
                .join("")
                .trim()
                .trim_end_matches("```")
                .to_string();

            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                return Some((tool, val));
            }
            // fallback: try the whole remaining as json
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(rest.trim()) {
                return Some((tool, val));
            }
        }
    }

    // 2) Very loose: look for a json object containing "tool" or first json blob
    if let Some(start) = t.find('{') {
        if let Some(end) = t[start..].rfind('}') {
            let candidate = &t[start..start + end + 1];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(candidate) {
                if let Some(name) = val.get("name").and_then(|v| v.as_str()) {
                    let args = val.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                    return Some((name.to_string(), args));
                }
                if let Some(name) = val.get("tool").and_then(|v| v.as_str()) {
                    let args = val.get("args").cloned().unwrap_or(serde_json::json!({}));
                    return Some((name.to_string(), args));
                }
            }
        }
    }

    None
}
