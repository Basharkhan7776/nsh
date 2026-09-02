// Key bindings module - configurable keyboard shortcuts
// Modify KEY_BINDINGS below to customize key combinations

use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::{LazyLock, Mutex};

use super::state::App;

// ══════════════════════════════════════════════════════════════════════════════
// CONFIGURATION - Modify key bindings here
// ══════════════════════════════════════════════════════════════════════════════

pub const KEY_BINDINGS: LazyLock<KeyBindings> = LazyLock::new(|| KeyBindings::default());

#[derive(Clone, Copy)]
pub struct KeyCombo {
    pub code: KeyCode,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyCombo {
    pub fn ctrl(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    pub fn alt(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            ctrl: false,
            alt: true,
            shift: false,
        }
    }

    pub fn ctrl_shift(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            ctrl: true,
            alt: false,
            shift: true,
        }
    }

    pub fn alt_code(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: true,
            shift: false,
        }
    }

    pub fn ctrl_code(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    pub fn code(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }
}

pub struct KeyBindings {
    pub move_line_start: KeyCombo,
    pub move_line_end: KeyCombo,
    pub move_word_left: KeyCombo,
    pub move_word_right: KeyCombo,
    pub move_char_left: KeyCombo,
    pub move_char_right: KeyCombo,
    pub delete_char_left: KeyCombo,
    pub delete_char_right: KeyCombo,
    pub delete_word_left: KeyCombo,
    pub delete_word_right: KeyCombo,
    pub delete_to_line_start: KeyCombo,
    pub delete_to_line_end: KeyCombo,
    pub delete_word: KeyCombo,
    pub yank: KeyCombo,
    pub copy: KeyCombo,
    pub paste: KeyCombo,
    pub history_up: KeyCombo,
    pub history_down: KeyCombo,
    pub suggestion_page_up: KeyCombo,
    pub suggestion_page_down: KeyCombo,
    pub complete: KeyCombo,
    pub interrupt: KeyCombo,
    pub eof: KeyCombo,
    pub cancel: KeyCombo,
    pub execute: KeyCombo,
    pub page_down_suggestions: KeyCombo,
    pub open_settings: KeyCombo,
    pub clear_screen: KeyCombo,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            // Movement
            move_line_start: KeyCombo::ctrl('a'),
            move_line_end: KeyCombo::ctrl('e'),
            // Defaults for documentation; get_action also accepts Ctrl+arrows.
            move_word_left: KeyCombo::ctrl_code(KeyCode::Left),
            move_word_right: KeyCombo::ctrl_code(KeyCode::Right),
            move_char_left: KeyCombo::code(KeyCode::Left),
            move_char_right: KeyCombo::code(KeyCode::Right),

            // Deletion
            delete_char_left: KeyCombo::code(KeyCode::Backspace),
            delete_char_right: KeyCombo::code(KeyCode::Delete),
            delete_word_left: KeyCombo::alt_code(KeyCode::Backspace),
            delete_word_right: KeyCombo::alt_code(KeyCode::Delete),
            delete_to_line_start: KeyCombo::ctrl('u'),
            delete_to_line_end: KeyCombo::ctrl('k'),
            delete_word: KeyCombo::ctrl('w'),

            // Clipboard
            yank: KeyCombo::ctrl('y'),
            copy: KeyCombo::ctrl_shift('c'),
            paste: KeyCombo::ctrl_shift('v'),

            // History
            history_up: KeyCombo::code(KeyCode::Up),
            history_down: KeyCombo::code(KeyCode::Down),
            suggestion_page_up: KeyCombo::code(KeyCode::PageUp),
            suggestion_page_down: KeyCombo::code(KeyCode::PageDown),
            page_down_suggestions: KeyCombo::ctrl('p'),

            // Completion
            complete: KeyCombo::code(KeyCode::Tab),

            // Special
            interrupt: KeyCombo::ctrl('c'),
            eof: KeyCombo::ctrl('d'),
            cancel: KeyCombo::code(KeyCode::Esc),
            execute: KeyCombo::code(KeyCode::Enter),
            open_settings: KeyCombo::ctrl(','),
            clear_screen: KeyCombo::ctrl('l'),
        }
    }
}

impl KeyBindings {
    pub fn matches(&self, key_code: KeyCode, modifiers: KeyModifiers, combo: &KeyCombo) -> bool {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let alt = modifiers.contains(KeyModifiers::ALT);
        let shift = modifiers.contains(KeyModifiers::SHIFT);

        key_code == combo.code && ctrl == combo.ctrl && alt == combo.alt && shift == combo.shift
    }
}

/// True for Backspace-like keys (terminals disagree on the encoding).
fn is_backspace(key_code: KeyCode) -> bool {
    matches!(
        key_code,
        KeyCode::Backspace | KeyCode::Char('\u{8}') | KeyCode::Char('\u{7f}')
    )
}

// ══════════════════════════════════════════════════════════════════════════════
// ACTION HANDLER - Processes key events into actions
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    None,
    Interrupt,
    Eof,
    Cancel,
    Execute,
    MoveLineStart,
    MoveLineEnd,
    MoveWordLeft,
    MoveWordRight,
    MoveCharLeft,
    MoveCharRight,
    DeleteCharLeft,
    DeleteCharRight,
    DeleteWordLeft,
    DeleteWordRight,
    DeleteToLineStart,
    DeleteToLineEnd,
    DeleteWord,
    Yank,
    Copy,
    Paste,
    HistoryUp,
    HistoryDown,
    SuggestionPageUp,
    SuggestionPageDown,
    PageDownSuggestions,
    Complete,
    InsertChar(char),
    OpenSettings,
    ClearScreen,
}

pub fn get_action(key_code: KeyCode, modifiers: KeyModifiers) -> Action {
    let bindings = &*KEY_BINDINGS;
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let alt = modifiers.contains(KeyModifiers::ALT);
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    // Super/Hyper alone shouldn't block navigation combos we care about.
    let has_meta = alt; // Alt / Meta (Esc-prefix also sets ALT in main)

    // ── Modifier + navigation (must run before bare arrow / backspace) ──────
    // Ctrl+←/→ and Alt+←/→ → word movement (common shell / editor bindings)
    if matches!(key_code, KeyCode::Left) && (ctrl || has_meta) {
        return Action::MoveWordLeft;
    }
    if matches!(key_code, KeyCode::Right) && (ctrl || has_meta) {
        return Action::MoveWordRight;
    }
    // Alt+b / Alt+f (emacs-style word move; often how terminals encode Alt+letter)
    if has_meta && !ctrl {
        if let KeyCode::Char(c) = key_code {
            match c.to_ascii_lowercase() {
                'b' => return Action::MoveWordLeft,
                'f' => return Action::MoveWordRight,
                'd' => return Action::DeleteWordRight,
                'h' | '\u{8}' | '\u{7f}' => return Action::DeleteWordLeft,
                _ => {}
            }
        }
    }
    // Alt/Ctrl+Backspace → delete previous word
    // Ctrl+Backspace is common on Linux GUIs; Alt+Backspace is classic readline.
    if is_backspace(key_code) && (has_meta || (ctrl && !shift)) {
        // Bare Ctrl+H is historically Backspace — only treat as word-delete when Alt
        // is held, or when it's real Backspace/Delete with Ctrl.
        if has_meta || matches!(key_code, KeyCode::Backspace) {
            return Action::DeleteWordLeft;
        }
    }
    // Alt/Ctrl+Delete → delete next word
    if matches!(key_code, KeyCode::Delete) && (has_meta || ctrl) {
        return Action::DeleteWordRight;
    }

    if bindings.matches(key_code, modifiers, &bindings.interrupt) {
        return Action::Interrupt;
    }
    if bindings.matches(key_code, modifiers, &bindings.eof) {
        return Action::Eof;
    }
    if bindings.matches(key_code, modifiers, &bindings.cancel) {
        return Action::Cancel;
    }
    if bindings.matches(key_code, modifiers, &bindings.execute) {
        return Action::Execute;
    }
    if bindings.matches(key_code, modifiers, &bindings.move_line_start) {
        return Action::MoveLineStart;
    }
    if bindings.matches(key_code, modifiers, &bindings.move_line_end) {
        return Action::MoveLineEnd;
    }
    // Home / End → start / end of input line
    if matches!(key_code, KeyCode::Home) {
        return Action::MoveLineStart;
    }
    if matches!(key_code, KeyCode::End) {
        return Action::MoveLineEnd;
    }
    // Bare arrows (no ctrl/alt)
    if matches!(key_code, KeyCode::Left) && !ctrl && !has_meta {
        return Action::MoveCharLeft;
    }
    if matches!(key_code, KeyCode::Right) && !ctrl && !has_meta {
        return Action::MoveCharRight;
    }
    // Bare backspace / delete
    if is_backspace(key_code) && !ctrl && !has_meta {
        return Action::DeleteCharLeft;
    }
    if matches!(key_code, KeyCode::Delete) && !ctrl && !has_meta {
        return Action::DeleteCharRight;
    }
    if bindings.matches(key_code, modifiers, &bindings.delete_to_line_start) {
        return Action::DeleteToLineStart;
    }
    if bindings.matches(key_code, modifiers, &bindings.delete_to_line_end) {
        return Action::DeleteToLineEnd;
    }
    if bindings.matches(key_code, modifiers, &bindings.delete_word) {
        return Action::DeleteWord;
    }
    if bindings.matches(key_code, modifiers, &bindings.yank) {
        return Action::Yank;
    }
    if bindings.matches(key_code, modifiers, &bindings.copy) {
        return Action::Copy;
    }
    if bindings.matches(key_code, modifiers, &bindings.paste) {
        return Action::Paste;
    }
    if bindings.matches(key_code, modifiers, &bindings.history_up) {
        return Action::HistoryUp;
    }
    if bindings.matches(key_code, modifiers, &bindings.history_down) {
        return Action::HistoryDown;
    }
    if bindings.matches(key_code, modifiers, &bindings.suggestion_page_up) {
        return Action::SuggestionPageUp;
    }
    if bindings.matches(key_code, modifiers, &bindings.suggestion_page_down) {
        return Action::SuggestionPageDown;
    }
    if bindings.matches(key_code, modifiers, &bindings.page_down_suggestions) {
        return Action::PageDownSuggestions;
    }
    if bindings.matches(key_code, modifiers, &bindings.complete) {
        return Action::Complete;
    }
    if bindings.matches(key_code, modifiers, &bindings.open_settings) {
        return Action::OpenSettings;
    }
    if bindings.matches(key_code, modifiers, &bindings.clear_screen) {
        return Action::ClearScreen;
    }

    // Character input (no modifiers or shift only for capitals)
    if let KeyCode::Char(c) = key_code {
        if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return Action::InsertChar(c);
        }
    }

    Action::None
}

// ══════════════════════════════════════════════════════════════════════════════
// CLIPBOARD - System clipboard operations
// ══════════════════════════════════════════════════════════════════════════════

static PERSISTENT_CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);

pub fn copy_to_clipboard(text: &str) -> bool {
    let mut guard = match PERSISTENT_CLIPBOARD.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    if guard.is_none() {
        *guard = arboard::Clipboard::new().ok();
    }
    if let Some(clipboard) = guard.as_mut() {
        let ok = clipboard.set_text(text).is_ok();
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            use arboard::{LinuxClipboardKind, SetExtLinux};
            let _ = clipboard.set().clipboard(LinuxClipboardKind::Primary).text(text);
        }
        ok
    } else {
        false
    }
}

/// Read paste text from the system clipboard.
///
/// On Linux also falls back to the primary selection (middle-click buffer),
/// which is what many terminals fill when you select text.
pub fn paste_from_clipboard() -> Option<String> {
    let mut guard = PERSISTENT_CLIPBOARD.lock().ok()?;
    if guard.is_none() {
        *guard = arboard::Clipboard::new().ok();
    }
    let clipboard = guard.as_mut()?;

    if let Ok(t) = clipboard.get_text() {
        let t = t.trim_end_matches(['\r', '\n']).to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }

    // Linux primary selection (X11 / some Wayland compositors)
    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    {
        use arboard::{GetExtLinux, LinuxClipboardKind};
        if let Ok(t) = clipboard
            .get()
            .clipboard(LinuxClipboardKind::Primary)
            .text()
        {
            let t = t.trim_end_matches(['\r', '\n']).to_string();
            if !t.is_empty() {
                return Some(t);
            }
        }
    }

    None
}

// ══════════════════════════════════════════════════════════════════════════════
// ACTION EXECUTION - Apply actions to app state
// ══════════════════════════════════════════════════════════════════════════════

pub fn execute_action(app: &mut App, action: Action) {
    match action {
        Action::None => {}

        Action::Interrupt => {
            app.current_input.clear();
            app.cursor_position = 0;
            app.history_index = None;
            app.show_suggestions = false;
        }

        Action::Eof => {
            // Handled in main loop
        }

        Action::Cancel => {
            app.show_suggestions = false;
            app.current_suggestions.clear();
        }

        Action::Execute => {
            // Handled in main loop
        }

        Action::MoveLineStart => {
            app.cursor_position = 0;
        }

        Action::MoveLineEnd => {
            app.cursor_position = app.current_input.len();
        }

        Action::MoveWordLeft => {
            app.cursor_position = app.word_start_backward();
        }

        Action::MoveWordRight => {
            app.cursor_position = app.word_start_forward();
        }

        Action::MoveCharLeft => {
            if app.cursor_position > 0 {
                app.cursor_position = prev_char_boundary(&app.current_input, app.cursor_position);
            }
        }

        Action::MoveCharRight => {
            if app.cursor_position < app.current_input.len() {
                app.cursor_position = next_char_boundary(&app.current_input, app.cursor_position);
            }
        }

        Action::DeleteCharLeft => {
            if app.cursor_position > 0 {
                let start = prev_char_boundary(&app.current_input, app.cursor_position);
                app.current_input.drain(start..app.cursor_position);
                app.cursor_position = start;
                app.history_index = None;
                app.update_suggestions();
            }
        }

        Action::DeleteCharRight => {
            if app.cursor_position < app.current_input.len() {
                let end = next_char_boundary(&app.current_input, app.cursor_position);
                app.current_input.drain(app.cursor_position..end);
                app.update_suggestions();
            }
        }

        Action::DeleteWordLeft => {
            app.delete_word_before();
        }

        Action::DeleteWordRight => {
            app.delete_word_after();
        }

        Action::DeleteToLineStart => {
            app.delete_to_line_start();
        }

        Action::DeleteToLineEnd => {
            app.delete_to_line_end();
        }

        Action::DeleteWord => {
            app.delete_word_before();
        }

        Action::Yank => {
            app.yank();
        }

        Action::Copy => {
            // Prefer current input; if empty, copy the most recent command output
            // so Ctrl+Shift+C is useful for grabbing shell/AI results.
            let text = if !app.current_input.is_empty() {
                if app.cursor_position < app.current_input.len() {
                    let sel_start = app.current_input[..app.cursor_position]
                        .rfind(' ')
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    let sel_end = app.current_input[app.cursor_position..]
                        .find(' ')
                        .map(|i| app.cursor_position + i)
                        .unwrap_or(app.current_input.len());
                    if sel_start < sel_end {
                        app.current_input[sel_start..sel_end].to_string()
                    } else {
                        app.current_input.clone()
                    }
                } else {
                    app.current_input.clone()
                }
            } else {
                app.entries
                    .iter()
                    .rev()
                    .find(|e| e.entry_type == super::state::EntryType::Output)
                    .map(|e| e.content.join("\n"))
                    .unwrap_or_default()
            };
            if !text.is_empty() {
                let _ = copy_to_clipboard(&text);
            }
        }

        Action::Paste => {
            if let Some(text) = paste_from_clipboard() {
                // Support multi-line paste; insert at cursor with correct UTF-8 length.
                app.current_input.insert_str(app.cursor_position, &text);
                app.cursor_position += text.len();
                app.history_index = None;
                app.update_suggestions();
            }
        }

        Action::HistoryUp => {
            // Handled in main loop
        }

        Action::HistoryDown => {
            // Handled in main loop
        }

        Action::SuggestionPageUp => {
            // Handled in main loop
        }

        Action::SuggestionPageDown => {
            // Handled in main loop
        }

        Action::PageDownSuggestions => {
            // Handled in main loop
        }

        Action::Complete => {
            // Handled in main loop
        }

        Action::OpenSettings => {
            // Handled in main loop
        }

        Action::ClearScreen => {
            // Handled in main loop (needs terminal access)
        }

        Action::InsertChar(c) => {
            app.current_input.insert(app.cursor_position, c);
            app.cursor_position += c.len_utf8();
            app.history_index = None;
            app.update_suggestions();
        }
    }
}

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 || pos > s.len() {
        return 0;
    }
    s[..pos]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    s[pos..]
        .chars()
        .next()
        .map(|c| pos + c.len_utf8())
        .unwrap_or(s.len())
}
