// Command completion and suggestion logic

use super::state::App;
use std::sync::LazyLock;

// Lazily loaded list of executable commands from PATH
pub static PATH_COMMANDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .filter_map(|dir| {
            std::fs::read_dir(dir).ok().map(|d| {
                d.filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
});

fn parse_dir_path(dir_part: &str) -> (String, String) {
    if dir_part.is_empty() {
        return (String::from("./"), String::new());
    }

    let dir_part = dir_part.replace("//", "/");

    if let Some(last_slash_idx) = dir_part.rfind('/') {
        let base = &dir_part[..last_slash_idx + 1];
        let prefix = &dir_part[last_slash_idx + 1..];
        (base.to_string(), prefix.to_string())
    } else {
        (String::from("./"), dir_part.to_string())
    }
}

// Update suggestions based on current input
pub fn update_suggestions(app: &mut App) {
    // Clear suggestions for empty input
    if app.current_input.is_empty() {
        app.current_suggestions.clear();
        app.show_suggestions = false;
        app.selected_suggestion = 0;
        app.suggestion_scroll_offset = 0;
        return;
    }

    let input_lower = app.current_input.to_lowercase();
    let mut suggestions: Vec<(String, String)> = Vec::new();

    // Directory completion for 'cd' command
    if app.current_input.starts_with("cd ") {
        let dir_part = app.current_input[3..].trim();
        let (base_dir, prefix) = parse_dir_path(dir_part);

        if let Ok(entries) = std::fs::read_dir(&base_dir) {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if is_dir {
                        let name_lower = name.to_lowercase();
                        if name_lower.starts_with(&prefix.to_lowercase()) || prefix.is_empty() {
                            let full_path = format!("{}{}/", base_dir, name);
                            let full_path = full_path.trim_start_matches("./").to_string();
                            let display_name = format!("{}/", name);
                            suggestions.push((full_path, display_name));
                        }
                    }
                }
            }
        }
        suggestions.sort_by(|a, b| a.1.cmp(&b.1));
    } else {
        // Command completion from PATH
        for cmd in PATH_COMMANDS.iter() {
            if cmd.to_lowercase().starts_with(&input_lower) {
                suggestions.push((cmd.clone(), cmd.clone()));
            }
        }

        // Add matching commands from history
        for entry in app.entries.iter().rev() {
            if entry.entry_type == super::state::EntryType::Command {
                if let Some(cmd) = entry.content.first() {
                    let cmd_lower = cmd.to_lowercase();
                    if cmd_lower.starts_with(&input_lower)
                        && !suggestions.iter().any(|s| s.0 == *cmd)
                    {
                        suggestions.push((cmd.clone(), cmd.clone()));
                    }
                }
            }
            if suggestions.len() >= 20 {
                break;
            }
        }
    }

    app.current_suggestions = suggestions;
    app.show_suggestions = !app.current_suggestions.is_empty();
    app.selected_suggestion = 0;
    app.suggestion_scroll_offset = 0;
}
