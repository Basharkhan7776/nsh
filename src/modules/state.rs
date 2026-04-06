// State management for terminal shell application

use super::completions::update_suggestions;
use super::config::{MAX_VISIBLE_SUGGESTIONS, VISIBLE_HISTORY_LINES};
use crate::ai::ProviderType;

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

    pub fn is_dropdown(&self) -> bool {
        matches!(self, SettingsField::Provider | SettingsField::Model)
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
    pub show_provider_dropdown: bool,
    pub show_model_dropdown: bool,
    pub dropdown_cursor: usize,
    pub editing_api_key: bool,
    pub editing_base_url: bool,
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
            show_provider_dropdown: false,
            show_model_dropdown: false,
            dropdown_cursor: 0,
            editing_api_key: false,
            editing_base_url: false,
        }
    }
}

// Application state
pub struct App {
    pub entries: Vec<Entry>,                        // All history entries
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
    pub show_settings: bool,                        // Settings dialog shown
    pub settings_state: SettingsState,              // Settings dialog state
    pub settings_cursor: usize,                     // Settings field cursor
    pub settings_input: String,                     // Input buffer for editing fields
}

impl App {
    // Initialize new application state
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
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
        }
    }

    // Add entry to history and update derived state
    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.push(entry);
        self.recalc_total_lines();
        self.scroll_to_bottom();
    }

    // Recalculate total line count from all entries
    pub fn recalc_total_lines(&mut self) {
        self.total_lines = self.entries.iter().map(|e| e.content.len()).sum();
    }

    // Clear all history
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

    // Extract command strings from history for navigation
    pub fn get_history_commands(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.entry_type == EntryType::Command)
            .filter_map(|e| e.content.first().cloned())
            .collect()
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

    // Move cursor to start of current word (going backward)
    pub fn word_start_backward(&self) -> usize {
        let input = &self.current_input[..self.cursor_position];
        if input.is_empty() {
            return 0;
        }

        let mut pos = input.len();
        let mut prev_was_word = false;

        for (i, c) in input.char_indices().rev() {
            let is_word_char = c.is_alphanumeric() || c == '_';
            if !prev_was_word && is_word_char && i > 0 {
                pos = i;
                break;
            }
            prev_was_word = is_word_char;
            pos = i;
        }

        pos
    }

    // Move cursor to end of current word (going forward)
    pub fn word_start_forward(&self) -> usize {
        let input = &self.current_input[self.cursor_position..];
        let mut pos = self.cursor_position;

        let mut chars = input.char_indices();
        let _ = chars.next(); // Skip current char

        let mut prev_was_word = false;
        for (i, c) in chars {
            let is_word_char = c.is_alphanumeric() || c == '_';
            if !prev_was_word && is_word_char {
                pos = i + self.cursor_position;
                break;
            }
            prev_was_word = is_word_char;
            pos = i + self.cursor_position + 1;
        }

        if pos > input.len() + self.cursor_position {
            pos = self.current_input.len();
        }

        pos
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

    pub fn settings_field(&self) -> SettingsField {
        SettingsField::from_index(self.settings_cursor)
    }

    pub fn settings_move_up(&mut self) {
        if self.settings_state.show_provider_dropdown || self.settings_state.show_model_dropdown {
            if self.settings_state.dropdown_cursor > 0 {
                self.settings_state.dropdown_cursor -= 1;
            }
        } else {
            if self.settings_cursor > 0 {
                self.settings_cursor -= 1;
            }
        }
    }

    pub fn settings_move_down(&mut self) {
        if self.settings_state.show_provider_dropdown || self.settings_state.show_model_dropdown {
            let max = if self.settings_state.show_provider_dropdown {
                ProviderType::count() - 1
            } else {
                self.settings_state.available_models.len().saturating_sub(1)
            };
            if self.settings_state.dropdown_cursor < max {
                self.settings_state.dropdown_cursor += 1;
            }
        } else {
            if self.settings_cursor < SettingsField::count() - 1 {
                self.settings_cursor += 1;
            }
        }
    }

    pub fn close_dropdowns(&mut self) {
        self.settings_state.show_provider_dropdown = false;
        self.settings_state.show_model_dropdown = false;
        self.settings_state.dropdown_cursor = 0;
    }

    pub fn settings_mask_api_key(key: &str) -> String {
        if key.is_empty() {
            "(empty)".to_string()
        } else {
            "••••••••••••••".to_string()
        }
    }
}
