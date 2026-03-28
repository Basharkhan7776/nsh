// Library root - public API exports

pub mod modules;

// Re-export public types and functions
pub use modules::commands::{execute_command, shorten_cwd};
pub use modules::completions::PATH_COMMANDS;
pub use modules::config::{
    COMMAND_FG, CWD_FG, INPUT_BG, INPUT_PROMPT_FG, MAX_VISIBLE_SUGGESTIONS, MOUSE_SCROLL_STEP,
    OUTPUT_BG, OUTPUT_FG, PROMPT_TEXT, SCROLL_STEP, SUGGESTION_INDICATOR_FG,
    SUGGESTION_SELECTED_BG, SUGGESTION_SELECTED_FG, SYSTEM_FG, VISIBLE_HISTORY_LINES,
};
pub use modules::keybindings;
pub use modules::render::render;
pub use modules::state::{App, Entry, EntryType};
