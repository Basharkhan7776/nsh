// NSH Shell - Binary Entry Point
// Terminal-based shell with TUI, autocompletion, and command history

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nsh::{
    keybindings::{execute_action, get_action, Action},
    render, App, Entry, EntryType, MAX_VISIBLE_SUGGESTIONS, MOUSE_SCROLL_STEP, SCROLL_STEP,
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> std::io::Result<()> {
    // Initialize terminal for alternate screen buffer
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize application state
    let mut app = App::new();
    app.add_entry(Entry {
        entry_type: EntryType::System,
        content: vec![
            "Welcome to nsh - AI-Powered Shell".to_string(),
            "Type 'help' for commands".to_string(),
            "Use Tab for autocomplete, Up/Down for history".to_string(),
        ],
        cwd: "~".to_string(),
    });

    let mut running = true;

    // Main event loop - processes keyboard and mouse input
    while running {
        render(&mut terminal, &app)?;

        loop {
            match event::poll(std::time::Duration::from_millis(50)) {
                Ok(true) => {
                    if let Ok(event) = event::read() {
                        match event {
                            Event::Key(key) => {
                                if key.kind != KeyEventKind::Press {
                                    continue;
                                }
                                let cwd = std::env::current_dir()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|_| "~".to_string());

                                let action = get_action(key.code, key.modifiers);

                                // Handle special actions that need cwd or custom logic
                                match action {
                                    Action::Interrupt => {
                                        app.add_entry(Entry {
                                            entry_type: EntryType::Command,
                                            content: vec!["^C".to_string()],
                                            cwd: cwd.clone(),
                                        });
                                        app.current_input.clear();
                                        app.cursor_position = 0;
                                        app.history_index = None;
                                        app.show_suggestions = false;
                                    }
                                    Action::Eof => {
                                        if app.current_input.is_empty() {
                                            running = false;
                                        }
                                    }
                                    Action::Execute => {
                                        let input = app.current_input.clone();
                                        if input == "exit" || input == "quit" {
                                            running = false;
                                        } else if !input.is_empty() {
                                            app.add_entry(Entry {
                                                entry_type: EntryType::Command,
                                                content: vec![input.clone()],
                                                cwd: cwd.clone(),
                                            });
                                            let output = nsh::execute_command(&input);
                                            if output.iter().any(|s| s == "__CLEAR__") {
                                                app.clear();
                                            } else if !output.is_empty() {
                                                app.add_entry(Entry {
                                                    entry_type: EntryType::Output,
                                                    content: output,
                                                    cwd: String::new(),
                                                });
                                            }
                                        }
                                        app.current_input.clear();
                                        app.cursor_position = 0;
                                        app.saved_input.clear();
                                        app.history_index = None;
                                        app.show_suggestions = false;
                                        app.current_suggestions.clear();
                                    }
                                    Action::Complete => {
                                        if !app.current_suggestions.is_empty() {
                                            if let Some(s) =
                                                app.current_suggestions.get(app.selected_suggestion)
                                            {
                                                if app.current_input.starts_with("cd ") {
                                                    app.current_input = format!("cd {}", s.0);
                                                } else {
                                                    app.current_input = s.1.clone();
                                                }
                                                app.cursor_position = app.current_input.len();
                                            }
                                            app.show_suggestions = false;
                                            app.current_suggestions.clear();
                                        } else if !app.current_input.is_empty() {
                                            app.update_suggestions();
                                            if app.current_suggestions.len() == 1 {
                                                if app.current_input.starts_with("cd ") {
                                                    app.current_input = format!(
                                                        "cd {}",
                                                        app.current_suggestions[0].0
                                                    );
                                                } else {
                                                    app.current_input =
                                                        app.current_suggestions[0].1.clone();
                                                }
                                                app.cursor_position = app.current_input.len();
                                                app.show_suggestions = false;
                                                app.current_suggestions.clear();
                                            }
                                        }
                                    }
                                    Action::HistoryUp => {
                                        if app.show_suggestions
                                            && !app.current_suggestions.is_empty()
                                        {
                                            if app.selected_suggestion > 0 {
                                                app.selected_suggestion -= 1;
                                            } else {
                                                app.selected_suggestion =
                                                    app.current_suggestions.len() - 1;
                                            }
                                            if app.selected_suggestion
                                                >= app.suggestion_scroll_offset
                                                    + MAX_VISIBLE_SUGGESTIONS
                                            {
                                                app.suggestion_scroll_offset = app
                                                    .selected_suggestion
                                                    .saturating_sub(MAX_VISIBLE_SUGGESTIONS - 1);
                                            }
                                            if app.selected_suggestion
                                                < app.suggestion_scroll_offset
                                            {
                                                app.suggestion_scroll_offset = app
                                                    .selected_suggestion
                                                    .saturating_sub(MAX_VISIBLE_SUGGESTIONS / 2);
                                            }
                                        } else {
                                            let commands = app.get_history_commands();
                                            if !commands.is_empty() {
                                                if app.history_index.is_none() {
                                                    app.saved_input = app.current_input.clone();
                                                    app.history_index = Some(commands.len() - 1);
                                                } else if let Some(idx) = app.history_index {
                                                    if idx > 0 {
                                                        app.history_index = Some(idx - 1);
                                                    }
                                                }
                                                if let Some(idx) = app.history_index {
                                                    if let Some(cmd) = commands.get(idx) {
                                                        app.current_input = cmd.clone();
                                                        app.cursor_position =
                                                            app.current_input.len();
                                                    }
                                                }
                                                app.show_suggestions = false;
                                            }
                                        }
                                    }
                                    Action::HistoryDown => {
                                        if app.show_suggestions
                                            && !app.current_suggestions.is_empty()
                                        {
                                            app.selected_suggestion = (app.selected_suggestion + 1)
                                                % app.current_suggestions.len();
                                            if app.selected_suggestion
                                                >= app.suggestion_scroll_offset
                                                    + MAX_VISIBLE_SUGGESTIONS
                                                && app.has_more_suggestions()
                                            {
                                                app.suggestion_scroll_offset = (app
                                                    .suggestion_scroll_offset
                                                    + MAX_VISIBLE_SUGGESTIONS)
                                                    .min(
                                                        app.current_suggestions
                                                            .len()
                                                            .saturating_sub(
                                                                MAX_VISIBLE_SUGGESTIONS,
                                                            ),
                                                    );
                                            }
                                        } else if let Some(idx) = app.history_index {
                                            let commands = app.get_history_commands();
                                            if idx < commands.len() - 1 {
                                                app.history_index = Some(idx + 1);
                                                if let Some(cmd) = commands.get(idx + 1) {
                                                    app.current_input = cmd.clone();
                                                    app.cursor_position = app.current_input.len();
                                                }
                                            } else {
                                                app.current_input = app.saved_input.clone();
                                                app.cursor_position = app.current_input.len();
                                                app.history_index = None;
                                            }
                                            app.show_suggestions = false;
                                        }
                                    }
                                    Action::SuggestionPageUp => {
                                        if app.show_suggestions && app.has_more_suggestions() {
                                            app.suggestion_page_up();
                                            app.selected_suggestion = app.suggestion_scroll_offset;
                                        } else {
                                            app.scroll_offset =
                                                app.scroll_offset.saturating_sub(SCROLL_STEP);
                                        }
                                    }
                                    Action::SuggestionPageDown | Action::PageDownSuggestions => {
                                        if app.show_suggestions && app.has_more_suggestions() {
                                            app.suggestion_page_down();
                                            app.selected_suggestion = app.suggestion_scroll_offset;
                                        } else {
                                            app.scroll_offset = (app.scroll_offset + SCROLL_STEP)
                                                .min(app.total_lines.saturating_sub(1));
                                        }
                                    }
                                    Action::Cancel => {
                                        app.show_suggestions = false;
                                        app.current_suggestions.clear();
                                    }
                                    _ => {
                                        // Use keybindings module for other actions
                                        execute_action(&mut app, action);
                                    }
                                }
                                break;
                            }

                            // Mouse input handling
                            Event::Mouse(mouse) => {
                                if mouse.kind == MouseEventKind::ScrollUp {
                                    if app.show_suggestions && app.has_more_suggestions() {
                                        app.suggestion_page_up();
                                    } else {
                                        app.scroll_offset =
                                            app.scroll_offset.saturating_sub(MOUSE_SCROLL_STEP);
                                    }
                                } else if mouse.kind == MouseEventKind::ScrollDown {
                                    if app.show_suggestions && app.has_more_suggestions() {
                                        app.suggestion_page_down();
                                    } else {
                                        app.scroll_offset = (app.scroll_offset + MOUSE_SCROLL_STEP)
                                            .min(app.total_lines.saturating_sub(1));
                                    }
                                }
                                break;
                            }
                            _ => {
                                break;
                            }
                        }
                    }
                }
                Ok(false) | Err(_) => {
                    break;
                }
            }
        }
    }

    // Cleanup - restore terminal to normal mode
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    println!("\nGoodbye!");
    Ok(())
}
