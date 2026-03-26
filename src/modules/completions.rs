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
    let mut suggestions: Vec<String> = Vec::new();

    // Directory completion for 'cd' command
    if app.current_input.starts_with("cd ") {
        let dir_part = app.current_input[3..].trim();
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if is_dir {
                        let name_lower = name.to_lowercase();
                        if name_lower.starts_with(&dir_part.to_lowercase()) || dir_part.is_empty() {
                            suggestions.push(name);
                        }
                    }
                }
            }
        }
        suggestions.sort();
    } else {
        // Command completion from PATH
        for cmd in PATH_COMMANDS.iter() {
            if cmd.to_lowercase().starts_with(&input_lower) {
                suggestions.push(cmd.clone());
            }
        }

        // Add matching commands from history
        for entry in app.entries.iter().rev() {
            if entry.entry_type == super::state::EntryType::Command {
                if let Some(cmd) = entry.content.first() {
                    if cmd.to_lowercase().starts_with(&input_lower) && !suggestions.contains(cmd) {
                        suggestions.push(cmd.clone());
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
