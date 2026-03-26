// State management for terminal shell application

use super::completions::update_suggestions;
use super::config::{MAX_VISIBLE_SUGGESTIONS, VISIBLE_HISTORY_LINES};

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

// Application state
pub struct App {
    pub entries: Vec<Entry>,              // All history entries
    pub current_input: String,            // Current input buffer
    pub cursor_position: usize,           // Cursor position in input
    pub scroll_offset: usize,             // Output scroll position
    pub total_lines: usize,               // Total lines in history
    pub current_suggestions: Vec<String>, // Active autocomplete suggestions
    pub show_suggestions: bool,           // Whether to display suggestions
    pub selected_suggestion: usize,       // Currently selected suggestion index
    pub suggestion_scroll_offset: usize,  // Suggestion page scroll position
    pub saved_input: String,              // Temporary storage for history navigation
    pub history_index: Option<usize>,     // Current position in command history
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
        self.current_suggestions[start..end].to_vec()
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
}
