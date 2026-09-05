// State management for terminal shell application

use super::completions::update_suggestions;
use super::config::{MAX_VISIBLE_SUGGESTIONS, VISIBLE_HISTORY_LINES};
use crate::ai::ProviderType;
use serde::{Deserialize, Serialize};

// Single line in command history
#[derive(Clone)]
pub struct Entry {
    pub entry_type: EntryType, // Type: command, output, or system message
    pub content: Vec<String>,  // Text content (may be multi-line)
    pub cwd: String,           // Current working directory when command was executed
}

// Entry type classification
#[derive(Clone, PartialEq)]
pub enum EntryType {
    Command, // User input line with prompt
    Output,  // Command execution result
    System,  // Welcome messages, help text, etc.
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Provider,
    Model,
    BaseUrl,
    ApiKey,
    Enable,
    Save,
    Cancel,
}

impl SettingsField {
    pub fn count() -> usize {
        7
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => SettingsField::Provider,
            1 => SettingsField::Model,
            2 => SettingsField::BaseUrl,
            3 => SettingsField::ApiKey,
            4 => SettingsField::Enable,
            5 => SettingsField::Save,
            6 => SettingsField::Cancel,
            _ => SettingsField::Provider,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    Home,
    Provider,
    Model,
    BaseUrl,
    ApiKey,
    Enable,
}

impl SettingsPage {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Home => " AI Settings ",
            Self::Provider => " Provider ",
            Self::Model => " Model ",
            Self::BaseUrl => " Base URL ",
            Self::ApiKey => " API Key ",
            Self::Enable => " Enable ",
        }
    }
}

#[derive(Clone)]
pub struct SettingsState {
    pub provider: ProviderType,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub api_key_original: String,
    pub enabled: bool,
    pub available_models: Vec<String>,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            provider: ProviderType::Ollama,
            model: "llama3.2:latest".to_string(),
            base_url: "http://localhost:11434".to_string(),
            api_key: String::new(),
            api_key_original: String::new(),
            enabled: false,
            available_models: vec!["llama3.2:latest".to_string()],
        }
    }
}

/// In-progress AI task (ask / do / plan / build) — drives the loading spinner UI.
#[derive(Clone)]
pub struct AiLoadingState {
    pub verb: String,     // "Asking", "Planning", …
    pub provider: String, // "gemini"
    pub model: String,    // "gemini-3.5-flash"
    pub frame: usize,     // spinner frame index
    pub current_step: usize,
    pub max_steps: usize,
    pub current_action: String,
}

impl AiLoadingState {
    pub const SPINNER: &'static [&'static str] =
        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    pub fn spinner_glyph(&self) -> &'static str {
        Self::SPINNER[self.frame % Self::SPINNER.len()]
    }

    pub fn status_line(&self) -> String {
        let action_str = if !self.current_action.is_empty() {
            format!(" | {}", self.current_action)
        } else if self.max_steps > 0 {
            format!(" [Step {}/{}]", self.current_step.max(1), self.max_steps)
        } else {
            String::new()
        };
        format!(
            "{} {} ({}){} — [Esc to stop]",
            self.spinner_glyph(),
            self.verb,
            self.model,
            action_str,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: (u16, u16), // (column, row)
    pub end: (u16, u16),   // (column, row)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SudoPromptMode {
    #[default]
    TuiModal,
    DesktopGui,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSession {
    pub goal: String,
    pub current_plan: String,
    pub iteration: usize,
}

// Application state
pub struct App {
    pub entries: Vec<Entry>,                        // All history entries (screen rows)
    pub command_history: Vec<String>,               // Persistent command history for suggestions & Up/Down
    pub current_input: String,                      // Current input buffer
    pub cursor_position: usize,                     // Cursor position in input
    pub scroll_offset: usize,                       // Output scroll position
    pub total_lines: usize,                         // Total lines in history
    pub current_suggestions: Vec<(String, String)>, // (full_path, display_name) for autocomplete
    pub show_suggestions: bool,                     // Whether to display suggestions
    pub selected_suggestion: usize,                 // Currently selected suggestion index
    pub suggestion_scroll_offset: usize,            // Suggestion page scroll position
    pub saved_input: String,                        // Temporary storage for history navigation
    pub history_index: Option<usize>,               // Current position in command history
    pub kill_ring: Vec<String>,                     // Kill ring for Ctrl+W / Ctrl+Y
    pub show_settings: bool,                        // Settings shown
    pub settings_state: SettingsState,              // Settings state
    pub settings_cursor: usize,                     // Settings field cursor (Home page index)
    pub settings_input: String,                     // Input buffer for editing fields
    pub settings_nav: Vec<SettingsPage>,            // Navigation stack
    pub settings_filter: String,                    // Search filter for settings
    pub settings_filter_active: bool,               // Whether filter input is active
    pub ai_loading: Option<AiLoadingState>,         // Active AI request spinner
    pub focus: Focus,                               // Current focus: Input or Output
    pub selection: Option<Selection>,               // Active text selection in output
    pub status_message: Option<String>,             // Temporary status notification
    pub show_sudo_prompt: bool,                     // Sudo password modal is active
    pub sudo_password: String,                      // Password input buffer (wiped on submit/cancel)
    pub pending_sudo_command: Option<String>,       // Original command waiting for sudo auth
    pub sudo_error: Option<String>,                 // Sudo auth error message
    pub sudo_prompt_mode: SudoPromptMode,           // Sudo prompt type (TuiModal / DesktopGui / Auto)
    pub input_scroll_x: usize,                      // Horizontal scroll offset for input
    pub active_plan_session: Option<PlanSession>,   // Active interactive plan review session
}

impl App {
    // Initialize new application state
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            command_history: Vec::new(),
            current_input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            total_lines: 0,
            current_suggestions: Vec::new(),
            show_suggestions: false,
            selected_suggestion: 0,
            suggestion_scroll_offset: 0,
            saved_input: String::new(),
            history_index: None,
            kill_ring: Vec::new(),
            show_settings: false,
            settings_state: SettingsState::default(),
            settings_cursor: 0,
            settings_input: String::new(),
            settings_nav: Vec::new(),
            settings_filter: String::new(),
            settings_filter_active: false,
            ai_loading: None,
            focus: Focus::Input,
            selection: None,
            status_message: None,
            show_sudo_prompt: false,
            sudo_password: String::new(),
            pending_sudo_command: None,
            sudo_error: None,
            sudo_prompt_mode: SudoPromptMode::default(),
            input_scroll_x: 0,
            active_plan_session: None,
        }
    }

    // Clear active plan session
    pub fn clear_plan_session(&mut self) {
        self.active_plan_session = None;
    }

    // Securely wipe and reset sudo password state
    pub fn clear_sudo_state(&mut self) {
        unsafe {
            let vec = self.sudo_password.as_mut_vec();
            for b in vec.iter_mut() {
                *b = 0;
            }
        }
        self.sudo_password.clear();
        self.show_sudo_prompt = false;
        self.pending_sudo_command = None;
        self.sudo_error = None;
    }

    // Load command history from local persistent storage
    pub fn load_history(&mut self) {
        if let Ok(storage) = crate::storage::LocalStorage::new() {
            self.command_history = storage.load_history();
        }
    }

    // Add command to history (ignoring consecutive duplicates, saving to persistent storage)
    pub fn add_command_history(&mut self, cmd: &str) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }

        // Avoid consecutive duplicate commands
        if self.command_history.last().map(|s| s.as_str()) == Some(trimmed) {
            return;
        }

        self.command_history.push(trimmed.to_string());
        if self.command_history.len() > crate::storage::local::MAX_HISTORY_ENTRIES {
            self.command_history.remove(0);
        }

        // Only persist to disk if command does NOT start with a leading space (privacy safeguard)
        if !cmd.starts_with(' ') {
            if let Ok(storage) = crate::storage::LocalStorage::new() {
                let _ = storage.append_history(trimmed);
            }
        }
    }

    // Add entry to history and update derived state
    pub fn add_entry(&mut self, entry: Entry) {
        if entry.entry_type == EntryType::Command && !entry.cwd.is_empty() {
            if let Some(first) = entry.content.first() {
                self.add_command_history(first);
            }
        }
        self.entries.push(entry);
        self.recalc_total_lines();
        self.scroll_to_bottom();
    }

    // Recalculate total line count from all entries
    pub fn recalc_total_lines(&mut self) {
        self.total_lines = self.entries.iter().map(|e| e.content.len()).sum();
    }

    // Clear all visual screen buffer (keeps command_history intact!)
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_lines = 0;
        self.scroll_offset = 0;
    }

    // Scroll to bottom of output
    pub fn scroll_to_bottom(&mut self) {
        let visible = self.visible_count();
        self.scroll_offset = self.total_lines.saturating_sub(visible);
    }

    // Visible line count in output area
    pub fn visible_count(&self) -> usize {
        VISIBLE_HISTORY_LINES
    }

    // Extract command strings from history for navigation (persists after clear)
    pub fn get_history_commands(&self) -> Vec<String> {
        self.command_history.clone()
    }

    // Get visible suggestion slice based on scroll offset
    pub fn visible_suggestions(&self) -> Vec<String> {
        let start = self.suggestion_scroll_offset;
        let end = (start + MAX_VISIBLE_SUGGESTIONS).min(self.current_suggestions.len());
        if start >= self.current_suggestions.len() {
            return vec![];
        }
        self.current_suggestions[start..end]
            .iter()
            .map(|s| s.1.clone())
            .collect()
    }

    // Check if more suggestions exist beyond visible range
    pub fn has_more_suggestions(&self) -> bool {
        self.suggestion_scroll_offset + MAX_VISIBLE_SUGGESTIONS < self.current_suggestions.len()
    }

    // Update suggestions based on current input
    pub fn update_suggestions(&mut self) {
        update_suggestions(self);
    }

    // Scroll suggestion list up by one page
    pub fn suggestion_page_up(&mut self) {
        if self.suggestion_scroll_offset > 0 {
            self.suggestion_scroll_offset = self
                .suggestion_scroll_offset
                .saturating_sub(MAX_VISIBLE_SUGGESTIONS);
            self.selected_suggestion = 0;
        }
    }

    // Scroll suggestion list down by one page
    pub fn suggestion_page_down(&mut self) {
        let max_scroll = self
            .current_suggestions
            .len()
            .saturating_sub(MAX_VISIBLE_SUGGESTIONS);
        self.suggestion_scroll_offset = self
            .suggestion_scroll_offset
            .saturating_add(MAX_VISIBLE_SUGGESTIONS)
            .min(max_scroll);
        self.selected_suggestion = 0;
    }

    /// Move to the start of the previous whitespace-delimited word (shell-style).
    pub fn word_start_backward(&self) -> usize {
        let cursor = self.cursor_position.min(self.current_input.len());
        if cursor == 0 {
            return 0;
        }
        let before = &self.current_input[..cursor];

        // Skip trailing whitespace before the cursor.
        let mut end = before.len();
        for (i, c) in before.char_indices().rev() {
            if !c.is_whitespace() {
                end = i + c.len_utf8();
                break;
            }
            if i == 0 {
                return 0;
            }
        }
        let trimmed = &before[..end];

        // Start of the word is just after the last whitespace.
        match trimmed
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace())
        {
            Some((i, c)) => i + c.len_utf8(),
            None => 0,
        }
    }

    /// Move to the start of the next whitespace-delimited word (shell-style).
    pub fn word_start_forward(&self) -> usize {
        let cursor = self.cursor_position.min(self.current_input.len());
        let rest = &self.current_input[cursor..];
        if rest.is_empty() {
            return self.current_input.len();
        }

        let mut iter = rest.char_indices().peekable();

        // Skip current word (non-whitespace).
        if iter.peek().is_some_and(|(_, c)| !c.is_whitespace()) {
            while iter.peek().is_some_and(|(_, c)| !c.is_whitespace()) {
                iter.next();
            }
        }

        // Skip whitespace that follows.
        while iter.peek().is_some_and(|(_, c)| c.is_whitespace()) {
            iter.next();
        }

        // Land on next word start, or end of input.
        match iter.peek() {
            Some(&(i, _)) => cursor + i,
            None => self.current_input.len(),
        }
    }

    // Delete word before cursor (bash-style, save to kill ring)
    pub fn delete_word_before(&mut self) {
        let word_start = self.word_start_backward();
        if word_start < self.cursor_position {
            let deleted = self.current_input[word_start..self.cursor_position].to_string();
            if !deleted.is_empty() {
                self.kill_ring.insert(0, deleted);
                if self.kill_ring.len() > 100 {
                    self.kill_ring.pop();
                }
            }
            self.current_input.drain(word_start..self.cursor_position);
            self.cursor_position = word_start;
            self.history_index = None;
            self.update_suggestions();
        }
    }

    // Delete word after cursor
    pub fn delete_word_after(&mut self) {
        let word_end = self.word_start_forward();
        if self.cursor_position < word_end {
            self.current_input.drain(self.cursor_position..word_end);
            self.update_suggestions();
        }
    }

    // Delete from cursor to line start
    pub fn delete_to_line_start(&mut self) {
        if self.cursor_position > 0 {
            let deleted = self.current_input[..self.cursor_position].to_string();
            if !deleted.is_empty() {
                self.kill_ring.insert(0, deleted);
                if self.kill_ring.len() > 100 {
                    self.kill_ring.pop();
                }
            }
            self.current_input.drain(..self.cursor_position);
            self.cursor_position = 0;
            self.history_index = None;
            self.update_suggestions();
        }
    }

    // Delete from cursor to line end
    pub fn delete_to_line_end(&mut self) {
        if self.cursor_position < self.current_input.len() {
            let deleted = self.current_input[self.cursor_position..].to_string();
            if !deleted.is_empty() {
                self.kill_ring.insert(0, deleted);
                if self.kill_ring.len() > 100 {
                    self.kill_ring.pop();
                }
            }
            self.current_input.drain(self.cursor_position..);
            self.update_suggestions();
        }
    }

    // Yank (paste) last killed text
    pub fn yank(&mut self) {
        if let Some(text) = self.kill_ring.first() {
            self.current_input.insert_str(self.cursor_position, text);
            self.cursor_position += text.len();
            self.history_index = None;
            self.update_suggestions();
        }
    }

    pub fn current_settings_page(&self) -> SettingsPage {
        self.settings_nav
            .last()
            .copied()
            .unwrap_or(SettingsPage::Home)
    }

    pub fn settings_push(&mut self, page: SettingsPage) {
        self.settings_nav.push(page);
        self.settings_cursor = 0;
    }

    pub fn settings_pop(&mut self) {
        self.settings_nav.pop();
        self.settings_cursor = 0;
    }

    pub fn settings_page_item_count(&self) -> usize {
        match self.current_settings_page() {
            SettingsPage::Home => SettingsField::count(),
            SettingsPage::Provider => ProviderType::count(),
            SettingsPage::Model => self.settings_state.available_models.len(),
            SettingsPage::Enable => 2,
            _ => 0,
        }
    }

    pub fn settings_move_up(&mut self) {
        if self.settings_cursor > 0 {
            self.settings_cursor -= 1;
        }
    }

    pub fn settings_move_down(&mut self) {
        let max = self.settings_page_item_count();
        if max > 0 && self.settings_cursor + 1 < max {
            self.settings_cursor += 1;
        }
    }

    pub fn settings_reset_filter(&mut self) {
        self.settings_filter.clear();
        self.settings_filter_active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_command_history_add_and_clear() {
        let mut app = App::new();
        assert!(app.command_history.is_empty());

        app.add_command_history("ls");
        app.add_command_history("cargo check");
        // Consecutive duplicate should be ignored
        app.add_command_history("cargo check");
        assert_eq!(app.command_history, vec!["ls", "cargo check"]);

        // Adding through add_entry
        app.add_entry(Entry {
            entry_type: EntryType::Command,
            content: vec!["cargo test".to_string()],
            cwd: "~".to_string(),
        });
        assert_eq!(app.get_history_commands(), vec!["ls", "cargo check", "cargo test"]);

        // Calling clear() wipes entries but leaves command_history intact
        app.clear();
        assert!(app.entries.is_empty());
        assert_eq!(app.get_history_commands(), vec!["ls", "cargo check", "cargo test"]);
    }

    #[test]
    fn test_clear_sudo_state() {
        let mut app = App::new();
        app.show_sudo_prompt = true;
        app.sudo_password = "mypassword".to_string();
        app.pending_sudo_command = Some("sudo ls".to_string());
        app.sudo_error = Some("failed".to_string());

        app.clear_sudo_state();
        assert!(!app.show_sudo_prompt);
        assert!(app.sudo_password.is_empty());
        assert!(app.pending_sudo_command.is_none());
        assert!(app.sudo_error.is_none());
    }
}
