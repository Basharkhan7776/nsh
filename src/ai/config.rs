use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: ProviderType,
    pub model: String,
    pub base_url: String,
    pub api_key: Option<String>,
    #[serde(default = "default_disabled")]
    pub enabled: bool,
}

fn default_disabled() -> bool {
    false
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Ollama,
    OpenAI,
    Anthropic,
    OpenAICompatible,
}

impl ProviderType {
    pub fn count() -> usize {
        4
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => ProviderType::Ollama,
            1 => ProviderType::OpenAI,
            2 => ProviderType::Anthropic,
            3 => ProviderType::OpenAICompatible,
            _ => ProviderType::Ollama,
        }
    }

    pub fn default_url(&self) -> &'static str {
        match self {
            ProviderType::Ollama => "http://localhost:11434",
            ProviderType::OpenAI => "https://api.openai.com/v1",
            ProviderType::Anthropic => "https://api.anthropic.com",
            ProviderType::OpenAICompatible => "http://localhost:11434/v1",
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: ProviderType::Ollama,
            model: "llama3".to_string(),
            base_url: "http://localhost:11434".to_string(),
            api_key: None,
            enabled: false,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Config not found")]
    NotFound,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Ollama => write!(f, "ollama"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::OpenAICompatible => write!(f, "openaicompatible"),
        }
    }
}
