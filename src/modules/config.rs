// Configuration constants for UI rendering and behavior

use ratatui::style::Color;

// Display constants
pub const MAX_VISIBLE_SUGGESTIONS: usize = 7; // Max items in suggestion popup
pub const VISIBLE_HISTORY_LINES: usize = 20; // Visible lines in output area
pub const SCROLL_STEP: usize = 5; // Lines scrolled per PageUp/PageDown
pub const MOUSE_SCROLL_STEP: usize = 3; // Lines scrolled per mouse wheel tick

// Prompt styling
pub const PROMPT_TEXT: &str = " $ "; // Input prompt prefix

// Output area colors (black background)
pub const OUTPUT_BG: Color = Color::Black;
pub const OUTPUT_FG: Color = Color::White;
pub const COMMAND_FG: Color = Color::DarkGray; // Command text color
pub const CWD_FG: Color = Color::Green; // Directory path color

// Input area colors (dark gray background)
pub const INPUT_BG: Color = Color::Rgb(30, 30, 30);
pub const INPUT_PROMPT_FG: Color = Color::Green;

// Suggestion popup colors
pub const SUGGESTION_SELECTED_BG: Color = Color::Blue;
pub const SUGGESTION_SELECTED_FG: Color = Color::White;
pub const SUGGESTION_INDICATOR_FG: Color = Color::DarkGray;

// System message color
pub const SYSTEM_FG: Color = Color::DarkGray;
