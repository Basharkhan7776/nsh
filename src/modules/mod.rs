// Module declarations for shell application

pub mod commands;
pub mod completions;
pub mod config;
pub mod keybindings;
pub mod render;
pub mod state;

pub use keybindings::{execute_action, get_action, Action};
