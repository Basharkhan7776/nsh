// Library root - public API exports

pub mod ai;
pub mod modules;
pub mod storage;
pub mod tools;
pub mod rag;

// Re-export public types and functions
pub use ai::{create_provider, fetch_models, AiConfig, AiError, AiProvider, ProviderType};
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
pub use storage::{LocalStorage, NshConfig, StorageError, VectorError, VectorStore};
pub use tools::{cat, execute_tool, get_tool_definitions, grep, ls, web_search};
