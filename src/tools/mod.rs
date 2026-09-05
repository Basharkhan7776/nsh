pub mod web_search;
pub mod terminal;

pub use terminal::{
    cat, cd, copy_path, delete_path, edit_file, exec_cmd, grep, ls, mkdir, move_path, touch, write_file,
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
            name: "edit_file".to_string(),
            description: "Modify an existing file by replacing a range of lines (1-indexed), applying a diff/patch, or replacing target text without rewriting the whole file. Conserves tokens and avoids overwriting unchanged code.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Target file path to edit"},
                    "start_line": {"type": "integer", "description": "1-based starting line number to replace (inclusive)"},
                    "end_line": {"type": "integer", "description": "1-based ending line number to replace (inclusive)"},
                    "replacement": {"type": "string", "description": "Replacement text to insert in place of the target lines (pass empty string to delete lines)"},
                    "target": {"type": "string", "description": "Optional exact text to search and replace, or verify matches the line range"},
                    "patch": {"type": "string", "description": "Optional unified diff hunk (+/- lines) to apply"}
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
            let path = args.get("path")
                .or_else(|| args.get("dir"))
                .or_else(|| args.get("directory"))
                .and_then(|v| v.as_str());
            let new_cwd = cd(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "cwd": new_cwd }))
        }

        "touch" => {
            let path = args.get("path")
                .or_else(|| args.get("file"))
                .or_else(|| args.get("filename"))
                .or_else(|| args.get("filepath"))
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            touch(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "ls" | "dir" => {
            let path = args.get("path")
                .or_else(|| args.get("dir"))
                .or_else(|| args.get("directory"))
                .and_then(|v| v.as_str());
            let entries = ls(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!(entries))
        }

        "cat" => {
            let path = args.get("path")
                .or_else(|| args.get("file"))
                .or_else(|| args.get("filename"))
                .or_else(|| args.get("filepath"))
                .and_then(|v| v.as_str())
                .ok_or("Missing path parameter")?;
            let content = cat(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "content": content }))
        }

        "grep" => {
            let pattern = args.get("pattern")
                .or_else(|| args.get("query"))
                .or_else(|| args.get("regex"))
                .and_then(|v| v.as_str())
                .ok_or("Missing pattern parameter")?;
            let path = args.get("path")
                .or_else(|| args.get("dir"))
                .or_else(|| args.get("directory"))
                .or_else(|| args.get("file"))
                .and_then(|v| v.as_str());
            let results = grep(pattern, path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "results": results }))
        }

        "write_file" | "write" => {
            let path = args.get("path")
                .or_else(|| args.get("file"))
                .or_else(|| args.get("filename"))
                .or_else(|| args.get("filepath"))
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            let content = args.get("content")
                .or_else(|| args.get("body"))
                .or_else(|| args.get("text"))
                .or_else(|| args.get("code"))
                .or_else(|| args.get("data"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            write_file(path, content).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "edit_file" | "replace_file_content" | "patch_file" | "modify_file" | "edit" => {
            let path = args.get("path")
                .or_else(|| args.get("file"))
                .or_else(|| args.get("filename"))
                .or_else(|| args.get("filepath"))
                .or_else(|| args.get("target_file"))
                .and_then(|v| v.as_str())
                .ok_or("Missing path parameter")?;

            let start_line = args.get("start_line")
                .or_else(|| args.get("start"))
                .or_else(|| args.get("from_line"))
                .or_else(|| args.get("line_start"))
                .and_then(|v| {
                    v.as_u64().map(|n| n as usize)
                        .or_else(|| v.as_str().and_then(|s| s.trim().parse::<usize>().ok()))
                });

            let end_line = args.get("end_line")
                .or_else(|| args.get("end"))
                .or_else(|| args.get("to_line"))
                .or_else(|| args.get("line_end"))
                .and_then(|v| {
                    v.as_u64().map(|n| n as usize)
                        .or_else(|| v.as_str().and_then(|s| s.trim().parse::<usize>().ok()))
                });

            let replacement = args.get("replacement")
                .or_else(|| args.get("replacement_content"))
                .or_else(|| args.get("content"))
                .or_else(|| args.get("text"))
                .or_else(|| args.get("new_content"))
                .or_else(|| args.get("code"))
                .or_else(|| args.get("new_str"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let target = args.get("target")
                .or_else(|| args.get("target_content"))
                .or_else(|| args.get("old_str"))
                .or_else(|| args.get("find"))
                .or_else(|| args.get("search"))
                .or_else(|| args.get("expected"))
                .and_then(|v| v.as_str());

            let patch = args.get("patch")
                .or_else(|| args.get("diff"))
                .or_else(|| args.get("hunk"))
                .and_then(|v| v.as_str());

            let summary = edit_file(path, start_line, end_line, replacement, target, patch)
                .map_err(|e| e.to_string())?;

            Ok(serde_json::json!({
                "ok": true,
                "path": path,
                "summary": summary,
            }))
        }

        "delete_path" | "delete" | "rm" => {
            let path = args.get("path")
                .or_else(|| args.get("file"))
                .or_else(|| args.get("dir"))
                .or_else(|| args.get("directory"))
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            delete_path(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "copy_path" | "copy" | "cp" => {
            let src = args.get("src")
                .or_else(|| args.get("source"))
                .or_else(|| args.get("from"))
                .and_then(|v| v.as_str())
                .ok_or("Missing src")?;
            let dst = args.get("dst")
                .or_else(|| args.get("dest"))
                .or_else(|| args.get("destination"))
                .or_else(|| args.get("to"))
                .and_then(|v| v.as_str())
                .ok_or("Missing dst")?;
            copy_path(src, dst).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "src": src, "dst": dst }))
        }

        "move_path" | "move" | "mv" => {
            let src = args.get("src")
                .or_else(|| args.get("source"))
                .or_else(|| args.get("from"))
                .and_then(|v| v.as_str())
                .ok_or("Missing src")?;
            let dst = args.get("dst")
                .or_else(|| args.get("dest"))
                .or_else(|| args.get("destination"))
                .or_else(|| args.get("to"))
                .and_then(|v| v.as_str())
                .ok_or("Missing dst")?;
            move_path(src, dst).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "src": src, "dst": dst }))
        }

        "mkdir" => {
            let path = args.get("path")
                .or_else(|| args.get("dir"))
                .or_else(|| args.get("directory"))
                .and_then(|v| v.as_str())
                .ok_or("Missing path")?;
            mkdir(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }

        "exec_cmd" | "run_command" | "exec" | "bash" | "sh" => {
            let cmd = args.get("cmd")
                .or_else(|| args.get("command"))
                .or_else(|| args.get("script"))
                .or_else(|| args.get("exec"))
                .or_else(|| args.get("input"))
                .and_then(|v| v.as_str())
                .ok_or("Missing cmd parameter")?;
            let out = exec_cmd(cmd).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "output": out }))
        }

        "web_search" => {
            let query = args.get("query")
                .or_else(|| args.get("q"))
                .or_else(|| args.get("search"))
                .and_then(|v| v.as_str())
                .ok_or("Missing query parameter")?;
            let results = web_search(query).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!(results))
        }

        _ => Err(format!("Unknown tool: {}", name)),
    }
}
