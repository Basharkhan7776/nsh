// State management for terminal shell application

use super::askpass::{secure_wipe_string, AuthPromptType};
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

#[derive(Debug, Clone)]
pub struct AuthModalState {
    pub is_active: bool,
    pub prompt_type: AuthPromptType,
    pub title: String,
    pub description: String,
    pub prompt_label: String,
    pub input_value: String,
    pub cursor_pos: usize,
    pub is_masked: bool,
    pub error_message: Option<String>,
    pub target_command: Option<String>,
}

impl Default for AuthModalState {
    fn default() -> Self {
        Self {
            is_active: false,
            prompt_type: AuthPromptType::GenericPassword,
            title: "Authentication Required".to_string(),
            description: String::new(),
            prompt_label: "Password:".to_string(),
            input_value: String::new(),
            cursor_pos: 0,
            is_masked: true,
            error_message: None,
            target_command: None,
        }
    }
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
    pub show_sudo_prompt: bool,                     // Sudo password modal is active (legacy/compat)
    pub sudo_password: String,                      // Password input buffer (wiped on submit/cancel)
    pub pending_sudo_command: Option<String>,       // Original command waiting for sudo auth
    pub sudo_error: Option<String>,                 // Sudo auth error message
    pub sudo_prompt_mode: SudoPromptMode,           // Sudo prompt type (TuiModal / DesktopGui / Auto)
    pub auth_modal: AuthModalState,                 // Universal authentication & password modal
    pub input_scroll_x: usize,                      // Horizontal scroll offset for input
    pub active_plan_session: Option<PlanSession>,   // Active interactive plan review session
    pub show_history_modal: bool,                   // Command history dialog modal is active
    pub history_modal_selected: usize,             // Currently selected history command index
    pub history_modal_scroll: usize,               // Scroll offset for history modal list
    pub history_modal_filter: String,              // Search filter for history dialog
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
            auth_modal: AuthModalState::default(),
            input_scroll_x: 0,
            active_plan_session: None,
            show_history_modal: false,
            history_modal_selected: 0,
            history_modal_scroll: 0,
            history_modal_filter: String::new(),
        }
    }

    // Clear active plan session
    pub fn clear_plan_session(&mut self) {
        self.active_plan_session = None;
    }

    // Securely wipe and reset sudo password state
    pub fn clear_sudo_state(&mut self) {
        secure_wipe_string(&mut self.sudo_password);
        self.show_sudo_prompt = false;
        self.pending_sudo_command = None;
        self.sudo_error = None;
    }

    /// Open universal authentication modal
    pub fn open_auth_modal(
        &mut self,
        prompt_type: AuthPromptType,
        title: &str,
        description: &str,
        label: &str,
        is_masked: bool,
        target_cmd: Option<String>,
    ) {
        secure_wipe_string(&mut self.auth_modal.input_value);
        self.auth_modal = AuthModalState {
            is_active: true,
            prompt_type,
            title: title.to_string(),
            description: description.to_string(),
            prompt_label: label.to_string(),
            input_value: String::new(),
            cursor_pos: 0,
            is_masked,
            error_message: None,
            target_command: target_cmd.clone(),
        };

        if prompt_type == AuthPromptType::SudoPassword {
            self.show_sudo_prompt = true;
            self.pending_sudo_command = target_cmd;
            self.sudo_password.clear();
            self.sudo_error = None;
        }
    }

    /// Close authentication modal and zeroize secret buffers
    pub fn close_auth_modal(&mut self) {
        secure_wipe_string(&mut self.auth_modal.input_value);
        self.auth_modal.is_active = false;
        self.auth_modal.cursor_pos = 0;
        self.auth_modal.error_message = None;
        self.auth_modal.target_command = None;
        self.clear_sudo_state();
    }

    /// Handle character input in auth modal
    pub fn auth_modal_input_char(&mut self, c: char) {
        if self.auth_modal.cursor_pos <= self.auth_modal.input_value.len() {
            self.auth_modal.input_value.insert(self.auth_modal.cursor_pos, c);
            self.auth_modal.cursor_pos += c.len_utf8();
            self.auth_modal.error_message = None;
            if self.auth_modal.prompt_type == AuthPromptType::SudoPassword {
                self.sudo_password = self.auth_modal.input_value.clone();
                self.sudo_error = None;
            }
        }
    }

    /// Handle backspace in auth modal
    pub fn auth_modal_backspace(&mut self) {
        if self.auth_modal.cursor_pos > 0 {
            let prev_idx = self.auth_modal.input_value[..self.auth_modal.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.auth_modal.input_value.remove(prev_idx);
            self.auth_modal.cursor_pos = prev_idx;
            self.auth_modal.error_message = None;
            if self.auth_modal.prompt_type == AuthPromptType::SudoPassword {
                self.sudo_password = self.auth_modal.input_value.clone();
                self.sudo_error = None;
            }
        }
    }

    /// Handle delete key in auth modal
    pub fn auth_modal_delete(&mut self) {
        if self.auth_modal.cursor_pos < self.auth_modal.input_value.len() {
            self.auth_modal.input_value.remove(self.auth_modal.cursor_pos);
            self.auth_modal.error_message = None;
            if self.auth_modal.prompt_type == AuthPromptType::SudoPassword {
                self.sudo_password = self.auth_modal.input_value.clone();
                self.sudo_error = None;
            }
        }
    }

    /// Move auth modal cursor left
    pub fn auth_modal_cursor_left(&mut self) {
        if self.auth_modal.cursor_pos > 0 {
            if let Some((idx, _)) = self.auth_modal.input_value[..self.auth_modal.cursor_pos]
                .char_indices()
                .next_back()
            {
                self.auth_modal.cursor_pos = idx;
            }
        }
    }

    /// Move auth modal cursor right
    pub fn auth_modal_cursor_right(&mut self) {
        if self.auth_modal.cursor_pos < self.auth_modal.input_value.len() {
            if let Some((_, ch)) = self.auth_modal.input_value[self.auth_modal.cursor_pos..]
                .char_indices()
                .next()
            {
                self.auth_modal.cursor_pos += ch.len_utf8();
            }
        }
    }

    /// Move auth modal cursor to beginning of line
    pub fn auth_modal_cursor_home(&mut self) {
        self.auth_modal.cursor_pos = 0;
    }

    /// Move auth modal cursor to end of line
    pub fn auth_modal_cursor_end(&mut self) {
        self.auth_modal.cursor_pos = self.auth_modal.input_value.len();
    }

    /// Clear input in auth modal (Ctrl+U)
    pub fn auth_modal_clear_input(&mut self) {
        secure_wipe_string(&mut self.auth_modal.input_value);
        self.auth_modal.cursor_pos = 0;
        self.auth_modal.error_message = None;
        if self.auth_modal.prompt_type == AuthPromptType::SudoPassword {
            self.sudo_password.clear();
            self.sudo_error = None;
        }
    }

    /// Delete word backward in auth modal (Ctrl+W)
    pub fn auth_modal_delete_word(&mut self) {
        if self.auth_modal.cursor_pos == 0 {
            return;
        }
        let s = &self.auth_modal.input_value[..self.auth_modal.cursor_pos];
        let trimmed = s.trim_end();
        let cut_idx = match trimmed.rfind(|c: char| c.is_whitespace() || c == '/' || c == '-' || c == '_') {
            Some(idx) => idx + 1,
            None => 0,
        };
        self.auth_modal.input_value.drain(cut_idx..self.auth_modal.cursor_pos);
        self.auth_modal.cursor_pos = cut_idx;
        self.auth_modal.error_message = None;
        if self.auth_modal.prompt_type == AuthPromptType::SudoPassword {
            self.sudo_password = self.auth_modal.input_value.clone();
            self.sudo_error = None;
        }
    }

    /// Paste text into auth modal
    pub fn auth_modal_paste(&mut self, text: &str) {
        let clean = text.trim_end_matches(['\r', '\n']);
        self.auth_modal.input_value.insert_str(self.auth_modal.cursor_pos, clean);
        self.auth_modal.cursor_pos += clean.len();
        self.auth_modal.error_message = None;
        if self.auth_modal.prompt_type == AuthPromptType::SudoPassword {
            self.sudo_password = self.auth_modal.input_value.clone();
            self.sudo_error = None;
        }
    }


    // Open command history dialog modal
    pub fn open_history_modal(&mut self) {
        self.show_history_modal = true;
        self.history_modal_filter.clear();
        self.history_modal_scroll = 0;
        let cmds = self.filtered_history_commands();
        if !cmds.is_empty() {
            // Highlight the latest command by default
            self.history_modal_selected = cmds.len().saturating_sub(1);
            self.current_input = cmds[self.history_modal_selected].clone();
            self.cursor_position = self.current_input.len();
            self.input_scroll_x = 0;
        } else {
            self.history_modal_selected = 0;
            self.current_input.clear();
            self.cursor_position = 0;
            self.input_scroll_x = 0;
        }
    }

    // Close command history dialog modal
    pub fn close_history_modal(&mut self) {
        self.show_history_modal = false;
        self.history_modal_filter.clear();
    }

    // Get list of commands matching current filter (ignoring bare "history" command)
    pub fn filtered_history_commands(&self) -> Vec<String> {
        let filter = self.history_modal_filter.trim().to_lowercase();
        self.command_history
            .iter()
            .filter(|cmd| {
                let trimmed = cmd.trim();
                if trimmed.eq_ignore_ascii_case("history") || trimmed.eq_ignore_ascii_case("/history") {
                    return false;
                }
                if filter.is_empty() {
                    true
                } else {
                    trimmed.to_lowercase().contains(&filter)
                }
            })
            .cloned()
            .collect()
    }

    // Move selection up in history modal (earlier command)
    pub fn history_modal_select_up(&mut self) {
        let cmds = self.filtered_history_commands();
        if cmds.is_empty() {
            return;
        }
        if self.history_modal_selected > 0 {
            self.history_modal_selected -= 1;
        }
        if let Some(cmd) = cmds.get(self.history_modal_selected) {
            self.current_input = cmd.clone();
            self.cursor_position = self.current_input.len();
            self.input_scroll_x = 0;
        }
    }

    // Move selection down in history modal (later command)
    pub fn history_modal_select_down(&mut self) {
        let cmds = self.filtered_history_commands();
        if cmds.is_empty() {
            return;
        }
        if self.history_modal_selected + 1 < cmds.len() {
            self.history_modal_selected += 1;
        }
        if let Some(cmd) = cmds.get(self.history_modal_selected) {
            self.current_input = cmd.clone();
            self.cursor_position = self.current_input.len();
            self.input_scroll_x = 0;
        }
    }

    // Confirm selection from history modal and close it
    pub fn history_modal_confirm(&mut self) {
        let cmds = self.filtered_history_commands();
        if let Some(cmd) = cmds.get(self.history_modal_selected) {
            self.current_input = cmd.clone();
            self.cursor_position = self.current_input.len();
            self.input_scroll_x = 0;
        }
        self.close_history_modal();
    }

    // Ensure the selected item is visible within the available height
    pub fn history_modal_adjust_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.history_modal_selected < self.history_modal_scroll {
            self.history_modal_scroll = self.history_modal_selected;
        } else if self.history_modal_selected >= self.history_modal_scroll + visible_height {
            self.history_modal_scroll = self.history_modal_selected + 1 - visible_height;
        }
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

    // Append a streaming chunk of live output to the active output entry, handling \r and \n
    pub fn append_live_output(&mut self, raw_chunk: &str) {
        if raw_chunk.is_empty() {
            return;
        }

        let chunk = super::commands::strip_ansi_escapes(raw_chunk);
        if chunk.is_empty() {
            return;
        }

        if self.entries.is_empty() {
            self.entries.push(Entry {
                entry_type: EntryType::Output,
                content: vec![String::new()],
                cwd: String::new(),
            });
        }
        let last_idx = self.entries.len() - 1;
        if self.entries[last_idx].entry_type != EntryType::Output {
            self.entries.push(Entry {
                entry_type: EntryType::Output,
                content: vec![String::new()],
                cwd: String::new(),
            });
        }

        let entry = self.entries.last_mut().unwrap();
        if entry.content.is_empty() {
            entry.content.push(String::new());
        }

        for c in chunk.chars() {
            if c == '\r' {
                // Carriage return: reset current line to allow progress bars to update in place
                if let Some(last_line) = entry.content.last_mut() {
                    last_line.clear();
                }
            } else if c == '\n' {
                // Newline: advance to next line
                entry.content.push(String::new());
            } else {
                if let Some(last_line) = entry.content.last_mut() {
                    last_line.push(c);
                }
            }
        }

        self.recalc_total_lines();
        self.scroll_to_bottom();
    }

    // Finalize live output entry when command terminates
    pub fn finalize_live_output(&mut self, status: std::process::ExitStatus) {
        if let Some(entry) = self.entries.last_mut() {
            if entry.entry_type == EntryType::Output {
                // Remove trailing empty line if text ended with newline
                if entry.content.len() > 1 && entry.content.last().is_some_and(|l| l.is_empty()) {
                    entry.content.pop();
                }

                // If nothing was printed:
                if entry.content.len() == 1 && entry.content[0].is_empty() {
                    if status.success() {
                        self.entries.pop();
                    } else if let Some(code) = status.code() {
                        if code != 130 {
                            entry.content[0] = format!("Process exited with status {}", code);
                        } else {
                            self.entries.pop();
                        }
                    } else {
                        self.entries.pop();
                    }
                }
            }
        }
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

    #[test]
    fn test_append_live_output_carriage_return_overwrite() {
        let mut app = App::new();
        // Progress bar simulation: "Writing: 10%\rWriting: 50%\rWriting: 100%\nDone\n"
        app.append_live_output("Writing: 10%\r");
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].content, vec![""]);

        app.append_live_output("Writing: 50%\r");
        assert_eq!(app.entries[0].content, vec![""]);

        app.append_live_output("Writing: 100%\nDone\n");
        assert_eq!(
            app.entries[0].content,
            vec!["Writing: 100%".to_string(), "Done".to_string(), "".to_string()]
        );

        // Finalize cleans trailing empty line
        let status = std::process::Command::new("true").status().unwrap();
        app.finalize_live_output(status);
        assert_eq!(
            app.entries[0].content,
            vec!["Writing: 100%".to_string(), "Done".to_string()]
        );
    }

    #[test]
    fn test_history_modal_lifecycle() {
        let mut app = App::new();
        app.add_command_history("git status");
        app.add_command_history("cargo build");
        app.add_command_history("npm test");

        // Open modal
        app.open_history_modal();
        assert!(app.show_history_modal);

        let cmds = app.filtered_history_commands();
        assert_eq!(cmds, vec!["git status", "cargo build", "npm test"]);

        // Default selection is the most recent command (npm test)
        assert_eq!(app.history_modal_selected, 2);
        assert_eq!(app.current_input, "npm test");

        // Navigate up (to cargo build)
        app.history_modal_select_up();
        assert_eq!(app.history_modal_selected, 1);
        assert_eq!(app.current_input, "cargo build");

        // Navigate up (to git status)
        app.history_modal_select_up();
        assert_eq!(app.history_modal_selected, 0);
        assert_eq!(app.current_input, "git status");

        // Navigate up at top boundary stays at 0
        app.history_modal_select_up();
        assert_eq!(app.history_modal_selected, 0);

        // Navigate down (back to cargo build)
        app.history_modal_select_down();
        assert_eq!(app.history_modal_selected, 1);
        assert_eq!(app.current_input, "cargo build");

        // Filtering
        app.history_modal_filter = "git".to_string();
        let filtered = app.filtered_history_commands();
        assert_eq!(filtered, vec!["git status"]);

        // Confirm selection: closes modal and leaves command in input
        app.history_modal_selected = 0;
        app.history_modal_confirm();
        assert!(!app.show_history_modal);
        assert_eq!(app.current_input, "git status");
    }

    #[test]
    fn test_auth_modal_lifecycle() {
        let mut app = App::new();
        assert!(!app.auth_modal.is_active);

        // Open SSH Key Passphrase modal
        app.open_auth_modal(
            AuthPromptType::SshKeyPassphrase,
            "SSH Key Passphrase",
            "Key: ~/.ssh/id_ed25519",
            "Passphrase:",
            true,
            Some("ssh user@vps".to_string()),
        );
        assert!(app.auth_modal.is_active);
        assert_eq!(app.auth_modal.title, "SSH Key Passphrase");
        assert_eq!(app.auth_modal.description, "Key: ~/.ssh/id_ed25519");
        assert_eq!(app.auth_modal.prompt_label, "Passphrase:");
        assert!(app.auth_modal.is_masked);

        // Typing characters
        app.auth_modal_input_char('s');
        app.auth_modal_input_char('e');
        app.auth_modal_input_char('c');
        app.auth_modal_input_char('r');
        app.auth_modal_input_char('e');
        app.auth_modal_input_char('t');
        assert_eq!(app.auth_modal.input_value, "secret");
        assert_eq!(app.auth_modal.cursor_pos, 6);

        // Cursor movement & deletion
        app.auth_modal_cursor_left();
        app.auth_modal_cursor_left();
        assert_eq!(app.auth_modal.cursor_pos, 4);
        app.auth_modal_backspace(); // deletes 'r'
        assert_eq!(app.auth_modal.input_value, "secet");
        assert_eq!(app.auth_modal.cursor_pos, 3);

        // Paste support
        app.auth_modal_paste("123");
        assert_eq!(app.auth_modal.input_value, "sec123et");

        // Clear input
        app.auth_modal_clear_input();
        assert_eq!(app.auth_modal.input_value, "");
        assert_eq!(app.auth_modal.cursor_pos, 0);

        // Close modal
        app.close_auth_modal();
        assert!(!app.auth_modal.is_active);
        assert_eq!(app.auth_modal.input_value, "");
    }
}

