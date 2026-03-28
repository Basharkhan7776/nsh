// Key bindings module - configurable keyboard shortcuts
// Modify KEY_BINDINGS below to customize key combinations

use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::LazyLock;

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
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            // Movement
            move_line_start: KeyCombo::ctrl('a'),
            move_line_end: KeyCombo::ctrl('e'),
            move_word_left: KeyCombo::alt_code(KeyCode::Left),
            move_word_right: KeyCombo::alt_code(KeyCode::Right),
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
}

pub fn get_action(key_code: KeyCode, modifiers: KeyModifiers) -> Action {
    let bindings = &*KEY_BINDINGS;

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
    if bindings.matches(key_code, modifiers, &bindings.move_word_left) {
        return Action::MoveWordLeft;
    }
    if bindings.matches(key_code, modifiers, &bindings.move_word_right) {
        return Action::MoveWordRight;
    }
    if bindings.matches(key_code, modifiers, &bindings.move_char_left) {
        return Action::MoveCharLeft;
    }
    if bindings.matches(key_code, modifiers, &bindings.move_char_right) {
        return Action::MoveCharRight;
    }
    if bindings.matches(key_code, modifiers, &bindings.delete_char_left) {
        return Action::DeleteCharLeft;
    }
    if bindings.matches(key_code, modifiers, &bindings.delete_char_right) {
        return Action::DeleteCharRight;
    }
    if bindings.matches(key_code, modifiers, &bindings.delete_word_left) {
        return Action::DeleteWordLeft;
    }
    if bindings.matches(key_code, modifiers, &bindings.delete_word_right) {
        return Action::DeleteWordRight;
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

    // Character input (no modifiers or shift only)
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

pub fn copy_to_clipboard(text: &str) -> bool {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        clipboard.set_text(text).is_ok()
    } else {
        false
    }
}

pub fn paste_from_clipboard() -> Option<String> {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        clipboard.get_text().ok()
    } else {
        None
    }
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
                app.cursor_position -= 1;
            }
        }

        Action::MoveCharRight => {
            if app.cursor_position < app.current_input.len() {
                app.cursor_position += 1;
            }
        }

        Action::DeleteCharLeft => {
            if app.cursor_position > 0 {
                app.cursor_position -= 1;
                app.current_input.remove(app.cursor_position);
                app.history_index = None;
                app.update_suggestions();
            }
        }

        Action::DeleteCharRight => {
            if app.cursor_position < app.current_input.len() {
                app.current_input.remove(app.cursor_position);
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
            let text = if app.cursor_position < app.current_input.len() {
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
            };
            let _ = copy_to_clipboard(&text);
        }

        Action::Paste => {
            if let Some(text) = paste_from_clipboard() {
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

        Action::InsertChar(c) => {
            app.current_input.insert(app.cursor_position, c);
            app.cursor_position += 1;
            app.history_index = None;
            app.update_suggestions();
        }
    }
}
