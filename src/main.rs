// NSH Shell - Binary Entry Point
// Terminal-based shell with TUI, autocompletion, and command history

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nsh::{render, App, Entry, EntryType, MAX_VISIBLE_SUGGESTIONS, MOUSE_SCROLL_STEP, SCROLL_STEP};
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

                                match key.code {
                                    // Character input
                                    KeyCode::Char(c) => {
                                        if key.modifiers.contains(event::KeyModifiers::CONTROL) {
                                            match c {
                                                'c' => {
                                                    // Interrupt - cancel input
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
                                                'd' => {
                                                    // EOF - exit shell
                                                    if app.current_input.is_empty() {
                                                        running = false;
                                                    }
                                                }
                                                'p' => {
                                                    // Page down in suggestions
                                                    if app.show_suggestions
                                                        && app.has_more_suggestions()
                                                    {
                                                        app.suggestion_page_down();
                                                    }
                                                }
                                                _ => {}
                                            }
                                        } else {
                                            // Regular character input
                                            app.current_input.insert(app.cursor_position, c);
                                            app.cursor_position += 1;
                                            app.history_index = None;
                                            app.update_suggestions();
                                        }
                                    }

                                    // Text editing
                                    KeyCode::Backspace => {
                                        if app.cursor_position > 0 {
                                            app.cursor_position -= 1;
                                            app.current_input.remove(app.cursor_position);
                                            app.history_index = None;
                                            app.update_suggestions();
                                        }
                                    }
                                    KeyCode::Delete => {
                                        if app.cursor_position < app.current_input.len() {
                                            app.current_input.remove(app.cursor_position);
                                            app.update_suggestions();
                                        }
                                    }
                                    KeyCode::Left => {
                                        if app.cursor_position > 0 {
                                            app.cursor_position -= 1;
                                        }
                                    }
                                    KeyCode::Right => {
                                        if app.cursor_position < app.current_input.len() {
                                            app.cursor_position += 1;
                                        }
                                    }

                                    // Command execution
                                    KeyCode::Enter => {
                                        let input = app.current_input.clone();

                                        // Handle built-in exit/quit commands
                                        if input == "exit" || input == "quit" {
                                            running = false;
                                        } else if !input.is_empty() {
                                            // Execute command and capture output
                                            app.add_entry(Entry {
                                                entry_type: EntryType::Command,
                                                content: vec![input.clone()],
                                                cwd: cwd.clone(),
                                            });

                                            let output = nsh::execute_command(&input);

                                            // Handle clear command
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

                                        // Reset input state
                                        app.current_input.clear();
                                        app.cursor_position = 0;
                                        app.saved_input.clear();
                                        app.history_index = None;
                                        app.show_suggestions = false;
                                        app.current_suggestions.clear();
                                    }

                                    // Autocompletion
                                    KeyCode::Tab => {
                                        if !app.current_suggestions.is_empty() {
                                            // Apply selected suggestion
                                            if let Some(s) =
                                                app.current_suggestions.get(app.selected_suggestion)
                                            {
                                                if app.current_input.starts_with("cd ") {
                                                    app.current_input = format!("cd {}", s);
                                                } else {
                                                    app.current_input = s.clone();
                                                }
                                                app.cursor_position = app.current_input.len();
                                            }
                                            app.show_suggestions = false;
                                            app.current_suggestions.clear();
                                        } else if !app.current_input.is_empty() {
                                            // Trigger suggestion lookup
                                            app.update_suggestions();
                                            if app.current_suggestions.len() == 1 {
                                                if app.current_input.starts_with("cd ") {
                                                    app.current_input = format!(
                                                        "cd {}",
                                                        app.current_suggestions[0]
                                                    );
                                                } else {
                                                    app.current_input =
                                                        app.current_suggestions[0].clone();
                                                }
                                                app.cursor_position = app.current_input.len();
                                                app.show_suggestions = false;
                                                app.current_suggestions.clear();
                                            }
                                        }
                                    }

                                    // History and suggestion navigation
                                    KeyCode::Up => {
                                        if app.show_suggestions
                                            && !app.current_suggestions.is_empty()
                                        {
                                            // Navigate suggestions
                                            if app.selected_suggestion > 0 {
                                                app.selected_suggestion -= 1;
                                            } else {
                                                app.selected_suggestion =
                                                    app.current_suggestions.len() - 1;
                                            }
                                            // Auto-scroll suggestion page
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
                                            // Navigate command history
                                            let commands = app.get_history_commands();
                                            if commands.is_empty() {
                                                break;
                                            }
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
                                                    app.cursor_position = app.current_input.len();
                                                }
                                            }
                                            app.show_suggestions = false;
                                        }
                                    }

                                    KeyCode::Down => {
                                        if app.show_suggestions
                                            && !app.current_suggestions.is_empty()
                                        {
                                            // Navigate suggestions
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
                                            // Navigate command history
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

                                    // Scrolling
                                    KeyCode::PageUp => {
                                        if app.show_suggestions && app.has_more_suggestions() {
                                            app.suggestion_page_up();
                                            app.selected_suggestion = app.suggestion_scroll_offset;
                                        } else {
                                            app.scroll_offset =
                                                app.scroll_offset.saturating_sub(SCROLL_STEP);
                                        }
                                    }
                                    KeyCode::PageDown => {
                                        if app.show_suggestions && app.has_more_suggestions() {
                                            app.suggestion_page_down();
                                            app.selected_suggestion = app.suggestion_scroll_offset;
                                        } else {
                                            app.scroll_offset = (app.scroll_offset + SCROLL_STEP)
                                                .min(app.total_lines.saturating_sub(1));
                                        }
                                    }
                                    KeyCode::Home => app.scroll_offset = 0,
                                    KeyCode::End => app.scroll_to_bottom(),

                                    // Close suggestions
                                    KeyCode::Esc => {
                                        app.show_suggestions = false;
                                        app.current_suggestions.clear();
                                    }
                                    _ => {}
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
