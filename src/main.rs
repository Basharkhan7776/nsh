// NSH Shell - Binary Entry Point
// Terminal-based shell with TUI, autocompletion, and command history

use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use nsh::{
    App, Entry, EntryType, Focus, LocalStorage, MAX_VISIBLE_SUGGESTIONS, MOUSE_SCROLL_STEP,
    SCROLL_STEP, Selection, SudoPromptMode,
    ai::{
        ProviderType,
        agent::{AiCommand, run_ai_command},
    },
    extract_selected_text, fetch_models,
    keybindings::{Action, execute_action, get_action},
    modules::state::{AiLoadingState, SettingsField, SettingsPage},
    rag::RagEngine,
    render,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::sync::mpsc;

/// Resolve Alt/Meta key combos.
///
/// Terminals often send Alt+Key as two events: `Esc` then `Key` within a few ms.
/// If a bare Esc arrives, peek for a following key and treat it as Alt+Key.
/// A lone Esc (no follow-up) is left as Esc.
fn resolve_key_with_alt_meta(code: KeyCode, modifiers: KeyModifiers) -> (KeyCode, KeyModifiers) {
    if code != KeyCode::Esc || modifiers != KeyModifiers::NONE {
        return (code, modifiers);
    }

    // Short window: real Esc is alone; Alt sequences arrive almost immediately.
    if event::poll(std::time::Duration::from_millis(25)).unwrap_or(false) {
        if let Ok(Event::Key(next)) = event::read() {
            if next.kind == KeyEventKind::Press {
                let mut mods = next.modifiers;
                mods.insert(KeyModifiers::ALT);
                return (next.code, mods);
            }
        }
    }

    (code, modifiers)
}

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
    config.ai.base_url = state.base_url.trim().to_string();
    // Trim so pasted keys with trailing newlines still work.
    let key = state.api_key_original.trim();
    config.ai.api_key = if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    };
    config.ai.enabled = state.enabled;

    let _ = storage.save_config(&config);
}

fn main() -> std::io::Result<()> {
    // Initialize terminal for alternate screen buffer with bracketed paste and mouse capture.
    // Mouse capture allows smooth mouse scrolling, click-to-focus on output, and drag text selection.
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize application state
    let mut app = App::new();
    app.load_history();
    if let Ok(storage) = LocalStorage::new() {
        let cfg = storage.load_or_create_config();
        app.sudo_prompt_mode = cfg.sudo_prompt_mode;
    }
    app.add_entry(Entry {
        entry_type: EntryType::System,
        content: vec![
            "Welcome to nsh - AI-Powered Shell".to_string(),
            "Type 'help' for commands".to_string(),
            "Use Tab for autocomplete / toggle focus, Up/Down for history, mouse wheel to scroll".to_string(),
            "Click output to focus & scroll. Drag to copy. Click input (or press 'i'/Enter) to type.".to_string(),
            "Type 'settings' or Ctrl+, for AI provider/model.  ask / do / plan / build + RAG ready.".to_string(),
        ],
        cwd: "~".to_string(),
    });

    let mut running = true;

    // Main event loop - processes keyboard and mouse input
    while running {
        render(&mut terminal, &mut app)?;

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

                                let (key_code, modifiers) =
                                    resolve_key_with_alt_meta(key.code, key.modifiers);
                                let action = get_action(key_code, modifiers);

                                // Global action: Open settings
                                if action == Action::OpenSettings {
                                    app.settings_state = load_settings_state();
                                    app.show_settings = true;
                                    app.settings_cursor = 0;
                                    app.settings_input.clear();
                                    app.settings_nav.clear();
                                    app.focus = Focus::Input;
                                    app.selection = None;

                                    // Fetch models for current provider
                                    let base_url = app.settings_state.base_url.clone();
                                    let provider = app.settings_state.provider;
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    app.settings_state.available_models =
                                        rt.block_on(fetch_models(provider, &base_url));
                                    break;
                                }

                                // Sudo password modal input handling
                                if app.show_sudo_prompt {
                                    match key_code {
                                        KeyCode::Esc => {
                                            app.clear_sudo_state();
                                            app.add_entry(Entry {
                                                entry_type: EntryType::Output,
                                                content: vec!["sudo: authentication cancelled".to_string()],
                                                cwd: String::new(),
                                            });
                                        }
                                        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                            app.clear_sudo_state();
                                            app.add_entry(Entry {
                                                entry_type: EntryType::Output,
                                                content: vec!["sudo: authentication cancelled".to_string()],
                                                cwd: String::new(),
                                            });
                                        }
                                        KeyCode::Backspace => {
                                            app.sudo_password.pop();
                                            app.sudo_error = None;
                                        }
                                        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                                            app.sudo_password.clear();
                                            app.sudo_error = None;
                                        }
                                        KeyCode::Enter => {
                                            let password = std::mem::take(&mut app.sudo_password);
                                            match nsh::validate_and_cache_sudo_password(&password) {
                                                Ok(()) => {
                                                    let cmd = app.pending_sudo_command.take().unwrap_or_default();
                                                    app.clear_sudo_state();
                                                    if !cmd.is_empty() {
                                                        let output = nsh::execute_command(&cmd);
                                                        if !output.is_empty() {
                                                            let cwd = std::env::current_dir()
                                                                .map(|p| p.to_string_lossy().to_string())
                                                                .unwrap_or_else(|_| "~".to_string());
                                                            app.add_entry(Entry {
                                                                entry_type: EntryType::Output,
                                                                content: output,
                                                                cwd,
                                                            });
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    app.sudo_error = Some(err);
                                                    app.sudo_password.clear();
                                                }
                                            }
                                        }
                                        KeyCode::Char(c) => {
                                            app.sudo_password.push(c);
                                            app.sudo_error = None;
                                        }
                                        _ => {}
                                    }
                                    break;
                                }

                                // Output focus mode (review / scroll / select)
                                if app.focus == Focus::Output {
                                    match key_code {
                                        KeyCode::Esc | KeyCode::Char('i') | KeyCode::Enter | KeyCode::Char('q') => {
                                            app.focus = Focus::Input;
                                            app.selection = None;
                                            app.status_message = None;
                                        }
                                        KeyCode::Up | KeyCode::Char('k') => {
                                            app.scroll_offset = app.scroll_offset.saturating_sub(1);
                                        }
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            let max_scroll = app.total_lines.saturating_sub(1);
                                            app.scroll_offset = (app.scroll_offset + 1).min(max_scroll);
                                        }
                                        KeyCode::PageUp => {
                                            app.scroll_offset = app.scroll_offset.saturating_sub(SCROLL_STEP);
                                        }
                                        KeyCode::PageDown => {
                                            let max_scroll = app.total_lines.saturating_sub(1);
                                            app.scroll_offset = (app.scroll_offset + SCROLL_STEP).min(max_scroll);
                                        }
                                        KeyCode::Char('g') => {
                                            app.scroll_offset = 0;
                                        }
                                        KeyCode::Char('G') => {
                                            app.scroll_offset = app.total_lines.saturating_sub(1);
                                        }
                                        KeyCode::Tab => {
                                            app.focus = Focus::Input;
                                            app.selection = None;
                                        }
                                        _ => {
                                            if action == Action::Interrupt || action == Action::Eof {
                                                running = false;
                                            }
                                        }
                                    }
                                    break;
                                }

                                // Input focus mode - Tab when input is empty toggles focus to output
                                if key_code == KeyCode::Tab && app.current_input.is_empty() && !app.show_suggestions {
                                    app.focus = Focus::Output;
                                    break;
                                }
                                app.status_message = None;

                                let cwd = std::env::current_dir()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|_| "~".to_string());

                                // Handle special actions that need cwd or custom logic
                                match action {
                                    Action::ClearScreen => {
                                        app.clear();
                                        terminal.clear()?;
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

                                            // AI command detection (bare and / forms) — use fresh config for live settings updates
                                            let trimmed = input.trim();
                                            let first =
                                                trimmed.split_whitespace().next().unwrap_or("");
                                            let cmd_word = first.trim_start_matches('/');
                                            if cmd_word == "index" || cmd_word == "/index" {
                                                // Lightweight RAG index (no full agent loop)
                                                let storage = LocalStorage::new()
                                                    .unwrap_or_else(|_| LocalStorage::default());
                                                let ai_cfg = storage.load_or_create_config().ai;
                                                let path_arg = trimmed
                                                    .split_once(char::is_whitespace)
                                                    .map(|(_, q)| q.trim())
                                                    .unwrap_or(".");
                                                let rt = tokio::runtime::Runtime::new().unwrap();
                                                let idx_res = rt.block_on(async {
                                                    match RagEngine::new_from_config(&storage, &ai_cfg, None).await {
                                                        Ok(mut engine) => {
                                                            // naive walk using same patterns as tools
                                                            let indexed = index_directory_simple(path_arg, &mut engine).await;
                                                            Ok(format!("Indexed {} documents into RAG from {}", indexed, path_arg))
                                                        }
                                                        Err(e) => Err(format!("RAG init failed: {}", e)),
                                                    }
                                                });
                                                let out = match idx_res {
                                                    Ok(msg) => vec![msg],
                                                    Err(e) => vec![e],
                                                };
                                                app.add_entry(Entry {
                                                    entry_type: EntryType::Output,
                                                    content: out,
                                                    cwd: String::new(),
                                                });
                                            } else if let Some(ai_cmd) =
                                                AiCommand::from_str(cmd_word)
                                            {
                                                // Extract query = everything after first token
                                                let query = trimmed
                                                    .split_once(char::is_whitespace)
                                                    .map(|(_, q)| q.trim().to_string())
                                                    .unwrap_or_default();

                                                let storage = LocalStorage::new()
                                                    .unwrap_or_else(|_| LocalStorage::default());
                                                let ai_cfg = storage.load_or_create_config().ai;

                                                let verb = match ai_cmd {
                                                    AiCommand::Ask => "Asking",
                                                    AiCommand::Do => "Doing",
                                                    AiCommand::Plan => "Planning",
                                                    AiCommand::Build => "Building",
                                                };
                                                let model = if ai_cfg.model.is_empty() {
                                                    "default".to_string()
                                                } else {
                                                    ai_cfg.model.clone()
                                                };

                                                // Clear input immediately so the loading UI is obvious.
                                                app.current_input.clear();
                                                app.cursor_position = 0;
                                                app.show_suggestions = false;
                                                app.current_suggestions.clear();

                                                // Start animated loading bar (must paint before work starts).
                                                app.ai_loading = Some(AiLoadingState {
                                                    verb: verb.to_string(),
                                                    provider: ai_cfg.provider.to_string(),
                                                    model: model.clone(),
                                                    frame: 0,
                                                });
                                                render(&mut terminal, &mut app)?;

                                                // Run AI on a background thread so the UI can spin.
                                                let (tx, rx) = mpsc::channel();
                                                let ai_cfg_bg = ai_cfg.clone();
                                                let query_bg = query.clone();
                                                let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                                                let cancel_bg = cancel_flag.clone();
                                                std::thread::spawn(move || {
                                                    let storage = LocalStorage::new()
                                                        .unwrap_or_else(|_| {
                                                            LocalStorage::default()
                                                        });
                                                    let rt = match tokio::runtime::Runtime::new() {
                                                        Ok(rt) => rt,
                                                        Err(e) => {
                                                            let _ = tx.send(Err(
                                                                nsh::AiError::Request(format!(
                                                                    "runtime: {e}"
                                                                )),
                                                            ));
                                                            return;
                                                        }
                                                    };
                                                    let res = rt.block_on(run_ai_command(
                                                         ai_cmd, &query_bg, ai_cfg_bg, &storage, Some(cancel_bg),
                                                    ));
                                                    let _ = tx.send(res);
                                                });

                                                // Animate spinner until the AI task finishes.
                                                loop {
                                                    if let Some(ref mut loading) = app.ai_loading {
                                                        loading.frame =
                                                            loading.frame.wrapping_add(1);
                                                    }
                                                    render(&mut terminal, &mut app)?;

                                                    match rx.try_recv() {
                                                        Ok(ai_out) => {
                                                            app.ai_loading = None;
                                                            let output_lines = match ai_out {
                                                                Ok(lines) => lines,
                                                                Err(e) => {
                                                                    vec![format!("AI error: {}", e)]
                                                                }
                                                            };
                                                            if !output_lines.is_empty() {
                                                                app.add_entry(Entry {
                                                                    entry_type: EntryType::Output,
                                                                    content: output_lines,
                                                                    cwd: String::new(),
                                                                });
                                                            } else if ai_cmd != AiCommand::Do {
                                                                app.add_entry(Entry {
                                                                    entry_type: EntryType::System,
                                                                    content: vec![format!(
                                                                        "[OK] {} finished (empty response)",
                                                                        verb
                                                                    )],
                                                                    cwd: String::new(),
                                                                });
                                                            }
                                                            break;
                                                        }
                                                        Err(mpsc::TryRecvError::Empty) => {
                                                            let mut user_cancelled = false;
                                                            while event::poll(
                                                                std::time::Duration::from_millis(0),
                                                            )
                                                            .unwrap_or(false)
                                                            {
                                                                if let Ok(Event::Key(key)) = event::read() {
                                                                    if key.code == KeyCode::Esc
                                                                        || (key.code == KeyCode::Char('c')
                                                                            && key.modifiers.contains(KeyModifiers::CONTROL))
                                                                    {
                                                                        user_cancelled = true;
                                                                        break;
                                                                    }
                                                                }
                                                            }

                                                            if user_cancelled {
                                                                cancel_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                                                                app.ai_loading = None;
                                                                app.add_entry(Entry {
                                                                    entry_type: EntryType::System,
                                                                    content: vec!["^C [Process stopped by Esc]".to_string()],
                                                                    cwd: String::new(),
                                                                });
                                                                break;
                                                            }

                                                            std::thread::sleep(
                                                                std::time::Duration::from_millis(
                                                                    60,
                                                                ),
                                                            );
                                                        }
                                                        Err(mpsc::TryRecvError::Disconnected) => {
                                                            app.ai_loading = None;
                                                            app.add_entry(Entry {
                                                                entry_type: EntryType::System,
                                                                content: vec![
                                                                    "AI task ended unexpectedly"
                                                                        .to_string(),
                                                                ],
                                                                cwd: String::new(),
                                                            });
                                                            break;
                                                        }
                                                    }
                                                }
                                            } else {
                                                if nsh::command_needs_sudo_password(&input) {
                                                    let use_gui = match app.sudo_prompt_mode {
                                                        SudoPromptMode::DesktopGui => true,
                                                        SudoPromptMode::Auto => {
                                                            std::env::var("DISPLAY").is_ok()
                                                                || std::env::var("WAYLAND_DISPLAY").is_ok()
                                                        }
                                                        SudoPromptMode::TuiModal => false,
                                                    };

                                                    let mut handled_via_gui = false;
                                                    if use_gui {
                                                        if let Some(pass) = nsh::prompt_gui_password("Authentication Required") {
                                                            handled_via_gui = true;
                                                            match nsh::validate_and_cache_sudo_password(&pass) {
                                                                Ok(()) => {
                                                                    let output = nsh::execute_command(&input);
                                                                    if !output.is_empty() {
                                                                        app.add_entry(Entry {
                                                                            entry_type: EntryType::Output,
                                                                            content: output,
                                                                            cwd: String::new(),
                                                                        });
                                                                    }
                                                                }
                                                                Err(err) => {
                                                                    app.add_entry(Entry {
                                                                        entry_type: EntryType::Output,
                                                                        content: vec![err],
                                                                        cwd: String::new(),
                                                                    });
                                                                }
                                                            }
                                                        }
                                                    }

                                                    if !handled_via_gui {
                                                        app.pending_sudo_command = Some(input.clone());
                                                        app.show_sudo_prompt = true;
                                                        app.sudo_password.clear();
                                                        app.sudo_error = None;
                                                    }
                                                } else {
                                                    let output = nsh::execute_command(&input);
                                                    if output.iter().any(|s| s == "__SETTINGS__") {
                                                        app.settings_state = load_settings_state();
                                                        app.show_settings = true;
                                                        app.settings_cursor = 0;
                                                        app.settings_input.clear();
                                                        app.settings_nav.clear();

                                                        let base_url =
                                                            app.settings_state.base_url.clone();
                                                        let provider = app.settings_state.provider;
                                                        let rt =
                                                            tokio::runtime::Runtime::new().unwrap();
                                                        app.settings_state.available_models = rt
                                                            .block_on(fetch_models(
                                                                provider, &base_url,
                                                            ));
                                                    } else if output.iter().any(|s| s == "__CLEAR__") {
                                                        app.clear();
                                                        terminal.clear()?;
                                                    } else if !output.is_empty() {
                                                        app.add_entry(Entry {
                                                            entry_type: EntryType::Output,
                                                            content: output,
                                                            cwd: String::new(),
                                                        });
                                                    }
                                                }
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
                                                app.current_input = s.0.clone();
                                                app.cursor_position = app.current_input.len();
                                            }
                                            app.show_suggestions = false;
                                            app.current_suggestions.clear();
                                        } else if !app.current_input.is_empty() {
                                            app.update_suggestions();
                                            if app.current_suggestions.len() == 1 {
                                                app.current_input =
                                                    app.current_suggestions[0].0.clone();
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

                            // Terminal paste (Ctrl+Shift+V / right-click paste with
                            // bracketed paste enabled). Prefer this path for API keys.
                            Event::Paste(text) => {
                                if app.show_settings {
                                    settings_apply_paste(&mut app, &text);
                                } else {
                                    // Insert into the shell input line.
                                    let cleaned = text.replace('\r', "");
                                    app.current_input.insert_str(app.cursor_position, &cleaned);
                                    app.cursor_position += cleaned.len();
                                    app.history_index = None;
                                    app.update_suggestions();
                                }
                                break;
                            }

                            // Mouse input handling
                            Event::Mouse(mouse) => {
                                // Settings mode: left click inside or outside modal
                                if app.show_settings
                                    && mouse.kind == MouseEventKind::Down(MouseButton::Left)
                                {
                                    let screen_size = terminal.size().unwrap_or_default();
                                    let screen_area = ratatui::layout::Rect::new(0, 0, screen_size.width, screen_size.height);
                                    let modal = nsh::compute_settings_modal_area(screen_area);

                                    // Click outside modal closes settings
                                    if mouse.column < modal.x
                                        || mouse.column >= modal.x + modal.width
                                        || mouse.row < modal.y
                                        || mouse.row >= modal.y + modal.height
                                    {
                                        app.show_settings = false;
                                        app.settings_reset_filter();
                                    } else if mouse.row == modal.y
                                        && mouse.column >= modal.x + modal.width.saturating_sub(10)
                                    {
                                        // Click on [Esc X] close button
                                        app.settings_pop();
                                        if app.settings_nav.is_empty() {
                                            app.show_settings = false;
                                            app.settings_reset_filter();
                                        }
                                    } else {
                                        let inner_y = modal.y + 1;
                                        let page = app.current_settings_page();
                                        match page {
                                            SettingsPage::Home => {
                                                if mouse.row == inner_y {
                                                    // Search bar clicked
                                                    app.settings_filter_active = true;
                                                } else {
                                                    let rel = mouse.row.saturating_sub(inner_y);
                                                    match rel {
                                                        3 => {
                                                            app.settings_cursor = 0;
                                                            settings_handle_enter(&mut app);
                                                        }
                                                        4 => {
                                                            app.settings_cursor = 1;
                                                            settings_handle_enter(&mut app);
                                                        }
                                                        6 => {
                                                            app.settings_cursor = 2;
                                                            settings_handle_enter(&mut app);
                                                        }
                                                        7 => {
                                                            app.settings_cursor = 3;
                                                            settings_handle_enter(&mut app);
                                                        }
                                                        9 => {
                                                            app.settings_state.enabled = !app.settings_state.enabled;
                                                        }
                                                        r if r >= modal.height.saturating_sub(3) => {
                                                            let mid_x = modal.x + modal.width / 2;
                                                            if mouse.column < mid_x {
                                                                app.settings_cursor = 5;
                                                                settings_handle_enter(&mut app);
                                                            } else {
                                                                app.settings_cursor = 6;
                                                                settings_handle_enter(&mut app);
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            SettingsPage::Provider => {
                                                let start_y = inner_y + 3;
                                                if mouse.row >= start_y {
                                                    let idx = ((mouse.row - start_y) / 2) as usize;
                                                    if idx < ProviderType::count() {
                                                        app.settings_cursor = idx;
                                                        settings_handle_enter(&mut app);
                                                    }
                                                }
                                            }
                                            SettingsPage::Model => {
                                                let start_y = inner_y + 3;
                                                if mouse.row >= start_y {
                                                    let idx = (mouse.row - start_y) as usize;
                                                    if idx < app.settings_state.available_models.len() {
                                                        app.settings_cursor = idx;
                                                        settings_handle_enter(&mut app);
                                                    }
                                                }
                                            }
                                            SettingsPage::Enable => {
                                                let start_y = inner_y + 4;
                                                if mouse.row >= start_y {
                                                    let idx = ((mouse.row - start_y) / 2) as usize;
                                                    if idx < 2 {
                                                        app.settings_cursor = idx;
                                                        settings_handle_enter(&mut app);
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                let terminal_size = terminal.size().unwrap_or_default();
                                let input_y = terminal_size.height.saturating_sub(if app.ai_loading.is_some() { 4 } else { 3 });

                                if !app.show_settings && !app.show_sudo_prompt {
                                    match mouse.kind {
                                        MouseEventKind::Down(MouseButton::Left) => {
                                            if mouse.row < input_y {
                                                // Clicked on output area -> focus output and begin selection
                                                app.focus = Focus::Output;
                                                app.status_message = None;
                                                app.selection = Some(Selection {
                                                    start: (mouse.column, mouse.row),
                                                    end: (mouse.column, mouse.row),
                                                });
                                            } else {
                                                // Clicked on input area -> focus input and clear selection
                                                app.focus = Focus::Input;
                                                app.selection = None;
                                                app.status_message = None;
                                            }
                                        }
                                        MouseEventKind::Drag(MouseButton::Left) => {
                                            if let Some(sel) = &mut app.selection {
                                                sel.end = (mouse.column, mouse.row);
                                            }
                                        }
                                        MouseEventKind::Up(MouseButton::Left) => {
                                            if let Some(sel) = app.selection {
                                                if sel.start != sel.end {
                                                    let wrap_width = terminal_size.width.saturating_sub(1) as usize;
                                                    let visible_height = input_y as usize;
                                                    if let Some(text) = extract_selected_text(&app, wrap_width, 0, visible_height) {
                                                        nsh::keybindings::copy_to_clipboard(&text);
                                                    }
                                                } else {
                                                    app.selection = None;
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                // Middle-click paste (primary selection) into editable settings.
                                if app.show_settings
                                    && mouse.kind == MouseEventKind::Down(MouseButton::Middle)
                                {
                                    if let Some(text) = nsh::keybindings::paste_from_clipboard() {
                                        settings_apply_paste(&mut app, &text);
                                    }
                                }
                                if mouse.kind == MouseEventKind::ScrollUp {
                                    if app.show_settings {
                                        app.settings_move_up();
                                    } else if app.show_suggestions && app.has_more_suggestions() {
                                        app.suggestion_page_up();
                                    } else {
                                        app.scroll_offset =
                                            app.scroll_offset.saturating_sub(MOUSE_SCROLL_STEP);
                                    }
                                } else if mouse.kind == MouseEventKind::ScrollDown {
                                    if app.show_settings {
                                        app.settings_move_down();
                                    } else if app.show_suggestions && app.has_more_suggestions() {
                                        app.suggestion_page_down();
                                    } else {
                                        let max_scroll = app.total_lines.saturating_sub(1);
                                        app.scroll_offset = (app.scroll_offset + MOUSE_SCROLL_STEP)
                                            .min(max_scroll);
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
    let _ = execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen
    );
    disable_raw_mode()?;
    println!("\nGoodbye!");
    Ok(())
}

/// Apply pasted text to the current settings field (API key / base URL).
fn settings_apply_paste(app: &mut App, text: &str) {
    use SettingsPage::*;

    // Strip surrounding whitespace/newlines from clipboard dumps.
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }

    match app.current_settings_page() {
        BaseUrl => {
            app.settings_state.base_url = text;
        }
        ApiKey => {
            // Replace entire key on paste (secret-field UX).
            app.settings_state.api_key = text.clone();
            app.settings_state.api_key_original = text;
        }
        _ => {
            // On other pages, ignore paste (or future: paste into search).
        }
    }
}

fn handle_settings_input(
    app: &mut App,
    key_code: crossterm::event::KeyCode,
    modifiers: crossterm::event::KeyModifiers,
) {
    use SettingsPage::*;
    use crossterm::event::KeyCode;

    let page = app.current_settings_page();
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);

    // Ctrl+S: Save and close immediately from anywhere in settings
    if ctrl && matches!(key_code, KeyCode::Char('s') | KeyCode::Char('S')) {
        save_settings_state(&app.settings_state);
        app.show_settings = false;
        app.settings_reset_filter();
        return;
    }

    // Clipboard paste via Ctrl+V / Ctrl+Shift+V (arboard).
    if ctrl && matches!(key_code, KeyCode::Char('v') | KeyCode::Char('V')) {
        if let Some(text) = nsh::keybindings::paste_from_clipboard() {
            settings_apply_paste(app, &text);
        }
        return;
    }

    // Ctrl+U clears the current editable field or search filter.
    if ctrl && matches!(key_code, KeyCode::Char('u') | KeyCode::Char('U')) {
        if app.settings_filter_active {
            app.settings_filter.clear();
        } else {
            match page {
                BaseUrl => app.settings_state.base_url.clear(),
                ApiKey => {
                    app.settings_state.api_key.clear();
                    app.settings_state.api_key_original.clear();
                }
                _ => {}
            }
        }
        return;
    }

    // Active filter mode handling on Home:
    if page == Home && app.settings_filter_active {
        match key_code {
            KeyCode::Esc | KeyCode::Enter => {
                app.settings_filter_active = false;
            }
            KeyCode::Backspace => {
                app.settings_filter.pop();
            }
            KeyCode::Char(c) if !ctrl => {
                app.settings_filter.push(c);
            }
            KeyCode::Up => app.settings_move_up(),
            KeyCode::Down => app.settings_move_down(),
            _ => {}
        }
        return;
    }

    match key_code {
        KeyCode::Esc => {
            if !app.settings_filter.is_empty() {
                app.settings_reset_filter();
            } else {
                app.settings_pop();
                if app.settings_nav.is_empty() {
                    app.show_settings = false;
                    app.settings_reset_filter();
                }
            }
        }
        KeyCode::Up | KeyCode::Char('k') if page != BaseUrl && page != ApiKey => {
            app.settings_move_up();
        }
        KeyCode::Down | KeyCode::Char('j') if page != BaseUrl && page != ApiKey => {
            app.settings_move_down();
        }
        KeyCode::Char('/') if page == Home && !ctrl => {
            app.settings_filter_active = true;
        }
        KeyCode::Char(' ') if page == Home && !ctrl => {
            // Space toggles Enable AI on Home
            if app.settings_cursor == 4 {
                app.settings_state.enabled = !app.settings_state.enabled;
            }
        }
        KeyCode::Up => app.settings_move_up(),
        KeyCode::Down => app.settings_move_down(),
        KeyCode::Enter => settings_handle_enter(app),
        KeyCode::Char(c) if !ctrl => match page {
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
                    app.settings_cursor = app.settings_state.provider.index();
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
            if app.settings_cursor >= ProviderType::count() {
                return;
            }
            let provider = ProviderType::from_index(app.settings_cursor);
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

// Simple non-recursive index helper for /index (reuses terminal read logic style).
async fn index_directory_simple(path: &str, engine: &mut RagEngine) -> usize {
    use std::fs;
    use std::path::Path;
    let root = Path::new(path);
    if !root.exists() {
        return 0;
    }
    let mut count = 0usize;
    let exts = [
        "rs", "toml", "md", "txt", "json", "yaml", "yml", "py", "js", "ts", "go", "sh",
    ];

    let walk = |dir: &Path| -> Vec<std::path::PathBuf> {
        let mut out = vec![];
        if let Ok(entries) = fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_file() {
                    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                        if exts.contains(&ext.to_lowercase().as_str()) {
                            out.push(p);
                        }
                    }
                }
            }
        }
        out
    };

    let files = if root.is_file() {
        vec![root.to_path_buf()]
    } else {
        walk(root)
    };

    for f in files.into_iter().take(50) {
        // limit for v1
        if let Ok(content) = fs::read_to_string(&f) {
            let id = f.display().to_string();
            let doc = nsh::rag::Document {
                id: id.clone(),
                content,
                source: id,
                metadata: None,
            };
            if engine.index_document(doc).await.is_ok() {
                count += 1;
            }
        }
    }
    count
}
