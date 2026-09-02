pub mod web_search;
pub mod terminal;

pub use terminal::{
    cat, cd, copy_path, delete_path, exec_cmd, grep, ls, mkdir, move_path, touch, write_file,
    FileEntry, TerminalError,
};
pub use web_search::{web_search, SearchResult, ToolError};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "cd".to_string(),
            description: "Change current working directory to path (defaults to home directory if path is omitted or ~)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Target directory path"}
                }
            }),
        },
        ToolDefinition {
            name: "touch".to_string(),
            description: "Create an empty file or update its timestamp. Creates parent directories if needed.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to touch"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "ls".to_string(),
            description: "List files and directories in a directory (defaults to current directory)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path to list"}
                }
            }),
        },
        ToolDefinition {
            name: "cat".to_string(),
            description: "Read contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write or overwrite a file with given content. Creates parent directories as needed.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path"},
                    "content": {"type": "string", "description": "Content to write (empty string if omitted)"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "mkdir".to_string(),
            description: "Create a directory and any necessary parent directories".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "delete_path".to_string(),
            description: "Delete a file or directory (recursive for directories)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to delete"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "copy_path".to_string(),
            description: "Copy a file or directory recursively".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "src": {"type": "string", "description": "Source path"},
                    "dst": {"type": "string", "description": "Destination path"}
                },
                "required": ["src", "dst"]
            }),
        },
        ToolDefinition {
            name: "move_path".to_string(),
            description: "Move or rename a file or directory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "src": {"type": "string", "description": "Source path"},
                    "dst": {"type": "string", "description": "Destination path"}
                },
                "required": ["src", "dst"]
            }),
        },
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search for regex pattern in files".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Regex pattern"},
                    "path": {"type": "string", "description": "Directory or file to search in"}
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "exec_cmd".to_string(),
            description: "Execute a shell command and capture its stdout and stderr".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "cmd": {"type": "string", "description": "Command string to run in shell"}
                },
                "required": ["cmd"]
            }),
        },
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web for information".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"}
                },
                "required": ["query"]
            }),
        },
    ]
}

pub async fn execute_tool(name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    match name {
        "cd" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let new_cwd = cd(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "cwd": new_cwd }))
        }

        "touch" => {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            touch(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "ls" | "dir" => {
            let path = args.get("path").and_then(|v| v.as_str());
            let entries = ls(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!(entries))
        }

        "cat" => {
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing path parameter")?;
            let content = cat(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "content": content }))
        }

        "grep" => {
            let pattern = args.get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("Missing pattern parameter")?;
            let path = args.get("path").and_then(|v| v.as_str());
            let results = grep(pattern, path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "results": results }))
        }

        "write_file" | "write" => {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            write_file(path, content).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "delete_path" | "delete" | "rm" => {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            delete_path(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "copy_path" | "copy" | "cp" => {
            let src = args.get("src").and_then(|v| v.as_str()).ok_or("Missing src")?;
            let dst = args.get("dst").and_then(|v| v.as_str()).ok_or("Missing dst")?;
            copy_path(src, dst).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "src": src, "dst": dst }))
        }

        "move_path" | "move" | "mv" => {
            let src = args.get("src").and_then(|v| v.as_str()).ok_or("Missing src")?;
            let dst = args.get("dst").and_then(|v| v.as_str()).ok_or("Missing dst")?;
            move_path(src, dst).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "src": src, "dst": dst }))
        }

        "mkdir" => {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            mkdir(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "exec_cmd" | "run_command" | "exec" | "bash" | "sh" => {
            let cmd = args.get("cmd")
                .or_else(|| args.get("command"))
                .and_then(|v| v.as_str())
                .ok_or("Missing cmd parameter")?;
            let out = exec_cmd(cmd).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "output": out }))
        }

        "web_search" => {
            let query = args.get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing query parameter")?;
            let results = web_search(query).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!(results))
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
