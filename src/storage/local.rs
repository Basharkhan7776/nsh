use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use thiserror::Error;

use crate::ai::config::AiConfig;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Path error: {0}")]
    Path(String),
}

pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub fn new() -> Result<Self, StorageError> {
        let base_path = Self::get_base_path()?;
        std::fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    fn get_base_path() -> Result<PathBuf, StorageError> {
        dirs::config_dir()
            .map(|p| p.join("nsh"))
            .ok_or_else(|| StorageError::Path("Could not find config directory".into()))
    }

    pub fn load<T: DeserializeOwned>(&self, filename: &str) -> Result<T, StorageError> {
        let path = self.base_path.join(filename);
        if !path.exists() {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            )));
        }
        let content = std::fs::read_to_string(&path)?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }

    pub fn save<T: Serialize>(&self, filename: &str, data: &T) -> Result<(), StorageError> {
        let path = self.base_path.join(filename);
        let content = serde_json::to_string_pretty(data)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn get_path(&self, filename: &str) -> PathBuf {
        self.base_path.join(filename)
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.base_path
    }

    pub fn load_config(&self) -> Result<NshConfig, StorageError> {
        self.load("config.json")
    }

    pub fn save_config(&self, config: &NshConfig) -> Result<(), StorageError> {
        self.save("config.json", config)
    }
    pub fn history_path(&self) -> PathBuf {
        self.base_path.join("history.txt")
    }

    pub fn load_history(&self) -> Vec<String> {
        let path = self.history_path();
        if !path.exists() {
            return Vec::new();
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let lines: Vec<String> = content
                    .lines()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
                if lines.len() > MAX_HISTORY_ENTRIES {
                    lines[lines.len() - MAX_HISTORY_ENTRIES..].to_vec()
                } else {
                    lines
                }
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn append_history(&self, cmd: &str) -> Result<(), StorageError> {
        use std::io::Write;
        let path = self.history_path();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "{}", cmd)?;
        Ok(())
    }

    pub fn save_history(&self, history: &[String]) -> Result<(), StorageError> {
        let path = self.history_path();
        let start = if history.len() > MAX_HISTORY_ENTRIES {
            history.len() - MAX_HISTORY_ENTRIES
        } else {
            0
        };
        let slice = &history[start..];
        let content = slice.join("\n") + "\n";
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub const MAX_HISTORY_ENTRIES: usize = 10_000;

impl Default for LocalStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create storage directory")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct NshConfig {
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub rag: RagConfig,
    #[serde(default)]
    pub sudo_prompt_mode: crate::modules::state::SudoPromptMode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RagConfig {
    pub embed_model: Option<String>,
    pub collection_name: String,
}

impl LocalStorage {
    pub fn load_or_create_config(&self) -> NshConfig {
        self.load_config().unwrap_or_else(|_| {
            let config = NshConfig::default();
            let _ = self.save_config(&config);
            config
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_storage_history_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("nsh_test_history_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let storage = LocalStorage { base_path: temp_dir.clone() };

        // Starts empty if no file
        let history = storage.load_history();
        assert!(history.is_empty());

        // Append commands
        storage.append_history("git status").unwrap();
        storage.append_history("cargo check").unwrap();
        storage.append_history("cargo test").unwrap();

        let loaded = storage.load_history();
        assert_eq!(loaded, vec!["git status", "cargo check", "cargo test"]);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
