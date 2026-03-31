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
}

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
