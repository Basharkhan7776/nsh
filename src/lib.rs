// Library root - public API exports

pub mod ai;
pub mod modules;
pub mod storage;
pub mod tools;
pub mod rag;

// Re-export public types and functions
pub use ai::{create_provider, fetch_models, AiConfig, AiError, AiProvider, ProviderType};
pub use ai::agent::{run_ai_command, AgentUpdate, AiCommand};
pub use modules::askpass::{
    classify_prompt, run_askpass_client, secure_wipe_string, AskPassPromptEvent, AskPassServer,
    AuthPromptType,
};
pub use modules::commands::{
    clean_interactive_input, command_needs_sudo_password, execute_command,
    execute_interactive_command, has_unquoted_shell_metachars, inject_askpass_env,
    is_fullscreen_tui, is_interactive_command, parse_command_line, prompt_gui_password,
    shorten_cwd, strip_ansi_escapes, validate_and_cache_sudo_password,
};
pub use modules::completions::PATH_COMMANDS;
pub use modules::config::{
    COMMAND_FG, CWD_FG, INPUT_BG, INPUT_PROMPT_FG, MAX_VISIBLE_SUGGESTIONS, MOUSE_SCROLL_STEP,
    OUTPUT_BG, OUTPUT_FG, PROMPT_TEXT, SCROLL_STEP, SUGGESTION_INDICATOR_FG,
    SUGGESTION_SELECTED_BG, SUGGESTION_SELECTED_FG, SYSTEM_FG, VISIBLE_HISTORY_LINES,
};
pub use modules::keybindings;
pub use modules::render::{
    compute_auth_modal_area, compute_history_modal_area, compute_settings_modal_area,
    compute_sudo_modal_area, extract_selected_text, render, render_auth_modal,
    render_history_modal, render_sudo_password_modal,
};
pub use modules::state::{
    App, AuthModalState, Entry, EntryType, Focus, PlanSession, Selection, SudoPromptMode,
};
pub use rag::{Document, RagEngine, RagError};
pub use storage::{LocalStorage, NshConfig, StorageError, VectorError, VectorStore};
pub use tools::{cat, copy_path, delete_path, execute_tool, get_tool_definitions, grep, ls, mkdir, move_path, web_search, write_file};
