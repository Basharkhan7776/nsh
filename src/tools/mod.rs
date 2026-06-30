pub mod web_search;
pub mod terminal;

pub use terminal::{cat, copy_path, delete_path, grep, ls, mkdir, move_path, write_file, FileEntry, TerminalError};
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
            name: "web_search".to_string(),
            description: "Search the web for information".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "cat".to_string(),
            description: "Read contents of a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "ls".to_string(),
            description: "List files in a directory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }),
        },
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search for pattern in files".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string"},
                    "path": {"type": "string"}
                },
                "required": ["pattern"]
            }),
        },
        // Write / mutate tools for do + build
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write or overwrite a file with the given content. Creates parent dirs.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "delete_path".to_string(),
            description: "Delete a file or directory (recursive for dirs)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "copy_path".to_string(),
            description: "Copy a file or directory (recursive)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "src": {"type": "string"},
                    "dst": {"type": "string"}
                },
                "required": ["src", "dst"]
            }),
        },
        ToolDefinition {
            name: "move_path".to_string(),
            description: "Move/rename a file or directory".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "src": {"type": "string"},
                    "dst": {"type": "string"}
                },
                "required": ["src", "dst"]
            }),
        },
        ToolDefinition {
            name: "mkdir".to_string(),
            description: "Create directory (and parents)".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        },
    ]
}

pub async fn execute_tool(name: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    match name {
        "web_search" => {
            let query = args.get("query")
                .and_then(|v| v.as_str())
                .ok_or("Missing query parameter")?;
            
            let results = web_search(query).await
                .map_err(|e| e.to_string())?;
            
            Ok(serde_json::json!(results))
        }
        
        "cat" => {
            let path = args.get("path")
                .and_then(|v| v.as_str())
                .ok_or("Missing path parameter")?;
            
            let content = cat(path).map_err(|e| e.to_string())?;
            
            Ok(serde_json::json!({ "content": content }))
        }
        
        "ls" => {
            let path = args.get("path").and_then(|v| v.as_str());
            
            let entries = ls(path).map_err(|e| e.to_string())?;
            
            Ok(serde_json::json!(entries))
        }
        
        "grep" => {
            let pattern = args.get("pattern")
                .and_then(|v| v.as_str())
                .ok_or("Missing pattern parameter")?;
            
            let path = args.get("path").and_then(|v| v.as_str());
            
            let results = grep(pattern, path).map_err(|e| e.to_string())?;
            
            Ok(serde_json::json!({ "results": results }))
        }

        // Mutating tools
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            let content = args.get("content").and_then(|v| v.as_str()).ok_or("Missing content")?;
            write_file(path, content).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }
        "delete_path" => {
            let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
            delete_path(path).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "path": path }))
        }
        "copy_path" => {
            let src = args.get("src").and_then(|v| v.as_str()).ok_or("Missing src")?;
            let dst = args.get("dst").and_then(|v| v.as_str()).ok_or("Missing dst")?;
            copy_path(src, dst).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "src": src, "dst": dst }))
        }
        "move_path" => {
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
        
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
