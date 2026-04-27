// NSH Shell - Binary Entry Point
// Terminal-based shell with TUI, autocompletion, and command history

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nsh::{
    ai::ProviderType,
    fetch_models,
    keybindings::{execute_action, get_action, Action},
    modules::state::{SettingsField, SettingsPage},
    render, App, Entry, EntryType, LocalStorage, MAX_VISIBLE_SUGGESTIONS, MOUSE_SCROLL_STEP,
    SCROLL_STEP,
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn load_settings_state() -> nsh::modules::state::SettingsState {
    let storage = LocalStorage::new().unwrap_or_else(|_| LocalStorage::default());
    let config = storage.load_or_create_config();

    let mut state = nsh::modules::state::SettingsState::default();
    state.provider = config.ai.provider;
    state.model = config.ai.model.clone();
    state.base_url = config.ai.base_url.clone();
    state.api_key_original = config.ai.api_key.clone().unwrap_or_default();
    state.api_key = state.api_key_original.clone();
    state.enabled = config.ai.enabled;
    state
}

fn save_settings_state(state: &nsh::modules::state::SettingsState) {
    let storage = LocalStorage::new().unwrap_or_else(|_| LocalStorage::default());
    let mut config = storage.load_or_create_config();

    config.ai.provider = state.provider;
    config.ai.model = state.model.clone();
    config.ai.base_url = state.base_url.clone();
    config.ai.api_key = if state.api_key_original.is_empty() {
        None
    } else {
        Some(state.api_key_original.clone())
    };
    config.ai.enabled = state.enabled;

    let _ = storage.save_config(&config);
}

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
            "Type 'settings' or press Ctrl+, for AI settings".to_string(),
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

                                // Handle settings mode
                                if app.show_settings {
                                    handle_settings_input(&mut app, key.code, key.modifiers);
                                    break;
                                }

                                let cwd = std::env::current_dir()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|_| "~".to_string());

                                let action = get_action(key.code, key.modifiers);

                                // Handle special actions that need cwd or custom logic
                                match action {
                                    Action::OpenSettings => {
                                        app.settings_state = load_settings_state();
                                        app.show_settings = true;
                                        app.settings_cursor = 0;
                                        app.settings_input.clear();
                                        app.settings_nav.clear();

                                        // Fetch models for current provider
                                        let base_url = app.settings_state.base_url.clone();
                                        let provider = app.settings_state.provider;
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        app.settings_state.available_models =
                                            rt.block_on(fetch_models(provider, &base_url));
                                    }
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
                                            if output.iter().any(|s| s == "__SETTINGS__") {
                                                app.settings_state = load_settings_state();
                                                app.show_settings = true;
                                                app.settings_cursor = 0;
                                                app.settings_input.clear();
                                                app.settings_nav.clear();

                                                let base_url = app.settings_state.base_url.clone();
                                                let provider = app.settings_state.provider;
                                                let rt = tokio::runtime::Runtime::new().unwrap();
                                                app.settings_state.available_models =
                                                    rt.block_on(fetch_models(provider, &base_url));
                                            } else if output.iter().any(|s| s == "__CLEAR__") {
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
                                // Settings mode: left click selects item
                                if app.show_settings
                                    && mouse.kind
                                        == MouseEventKind::Down(MouseButton::Left)
                                {
                                    let page = app.current_settings_page();
                                    let y_offset: u16 = match page {
                                        SettingsPage::Home
                                        | SettingsPage::Provider
                                        | SettingsPage::Model => 2,
                                        SettingsPage::Enable => 4,
                                        _ => 0,
                                    };
                                    if y_offset > 0
                                        && let Some(row) =
                                            mouse.row.checked_sub(y_offset)
                                    {
                                        let idx = row as usize;
                                        let count = app.settings_page_item_count();
                                        if count > 0 && idx < count {
                                            app.settings_cursor = idx;
                                            settings_handle_enter(&mut app);
                                        }
                                    }
                                }
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

fn handle_settings_input(
    app: &mut App,
    key_code: crossterm::event::KeyCode,
    _modifiers: crossterm::event::KeyModifiers,
) {
    use crossterm::event::KeyCode;
    use SettingsPage::*;

    let page = app.current_settings_page();

    match key_code {
        KeyCode::Esc => {
            app.settings_pop();
            if app.settings_nav.is_empty() {
                app.show_settings = false;
            }
        }
        KeyCode::Up => app.settings_move_up(),
        KeyCode::Down => app.settings_move_down(),
        KeyCode::Enter => settings_handle_enter(app),
        KeyCode::Char(c) => match page {
            BaseUrl => {
                app.settings_state.base_url.push(c);
            }
            ApiKey => {
                app.settings_state.api_key.push(c);
                app.settings_state.api_key_original = app.settings_state.api_key.clone();
            }
            _ => {}
        },
        KeyCode::Backspace => match page {
            BaseUrl => {
                app.settings_state.base_url.pop();
            }
            ApiKey => {
                app.settings_state.api_key.pop();
                app.settings_state.api_key_original = app.settings_state.api_key.clone();
            }
            _ => {}
        },
        _ => {}
    }
}

fn settings_handle_enter(app: &mut App) {
    use SettingsPage::*;

    let page = app.current_settings_page();
    match page {
        Home => {
            let field = SettingsField::from_index(app.settings_cursor);
            match field {
                SettingsField::Provider => {
                    app.settings_push(Provider);
                    app.settings_cursor = match app.settings_state.provider {
                        ProviderType::Ollama => 0,
                        ProviderType::OpenAI => 1,
                        ProviderType::Anthropic => 2,
                        ProviderType::OpenAICompatible => 3,
                    };
                }
                SettingsField::Model => {
                    if !app.settings_state.available_models.is_empty() {
                        app.settings_push(Model);
                        if let Some(idx) = app
                            .settings_state
                            .available_models
                            .iter()
                            .position(|m| m == &app.settings_state.model)
                        {
                            app.settings_cursor = idx;
                        } else {
                            app.settings_cursor = 0;
                        }
                    }
                }
                SettingsField::BaseUrl => {
                    app.settings_push(BaseUrl);
                }
                SettingsField::ApiKey => {
                    app.settings_push(ApiKey);
                }
                SettingsField::Enable => {
                    app.settings_push(Enable);
                    app.settings_cursor = if app.settings_state.enabled { 0 } else { 1 };
                }
                SettingsField::Save => {
                    save_settings_state(&app.settings_state);
                    app.show_settings = false;
                }
                SettingsField::Cancel => {
                    app.show_settings = false;
                }
            }
        }
        Provider => {
            let provider = match app.settings_cursor {
                0 => ProviderType::Ollama,
                1 => ProviderType::OpenAI,
                2 => ProviderType::Anthropic,
                3 => ProviderType::OpenAICompatible,
                _ => return,
            };
            app.settings_state.provider = provider;
            app.settings_state.base_url = provider.default_url().to_string();
            app.settings_state.model = String::new();

            let rt = tokio::runtime::Runtime::new().unwrap();
            app.settings_state.available_models =
                rt.block_on(fetch_models(provider, &app.settings_state.base_url));
            if !app.settings_state.available_models.is_empty() {
                app.settings_state.model = app.settings_state.available_models[0].clone();
            }

            app.settings_pop();
        }
        Model => {
            if let Some(model) = app.settings_state.available_models.get(app.settings_cursor) {
                app.settings_state.model = model.clone();
            }
            app.settings_pop();
        }
        BaseUrl => {
            app.settings_pop();
        }
        ApiKey => {
            app.settings_state.api_key_original = app.settings_state.api_key.clone();
            app.settings_pop();
        }
        Enable => {
            app.settings_state.enabled = app.settings_cursor == 0;
            app.settings_pop();
        }
    }
}
