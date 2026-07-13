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
    Gemini,
    OpenRouter,
    OpenAICompatible,
}

impl ProviderType {
    pub fn count() -> usize {
        6
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => ProviderType::Ollama,
            1 => ProviderType::OpenAI,
            2 => ProviderType::Anthropic,
            3 => ProviderType::Gemini,
            4 => ProviderType::OpenRouter,
            5 => ProviderType::OpenAICompatible,
            _ => ProviderType::Ollama,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            ProviderType::Ollama => 0,
            ProviderType::OpenAI => 1,
            ProviderType::Anthropic => 2,
            ProviderType::Gemini => 3,
            ProviderType::OpenRouter => 4,
            ProviderType::OpenAICompatible => 5,
        }
    }

    pub fn default_url(&self) -> &'static str {
        match self {
            ProviderType::Ollama => "http://localhost:11434",
            ProviderType::OpenAI => "https://api.openai.com/v1",
            ProviderType::Anthropic => "https://api.anthropic.com",
            // OpenAI-compatible Gemini endpoint
            ProviderType::Gemini => "https://generativelanguage.googleapis.com/v1beta/openai/",
            ProviderType::OpenRouter => "https://openrouter.ai/api/v1",
            ProviderType::OpenAICompatible => "http://localhost:11434/v1",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::Ollama => "Ollama",
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Anthropic => "Anthropic",
            ProviderType::Gemini => "Gemini",
            ProviderType::OpenRouter => "OpenRouter",
            ProviderType::OpenAICompatible => "OpenAI Compatible",
        }
    }

    /// Whether this provider needs an API key for cloud auth.
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, ProviderType::Ollama)
    }

    /// Env vars checked (in order) when config has no usable key.
    pub fn api_key_env_vars(&self) -> &'static [&'static str] {
        match self {
            ProviderType::Ollama => &[],
            ProviderType::OpenAI => &["OPENAI_API_KEY", "NSH_API_KEY"],
            ProviderType::Anthropic => &["ANTHROPIC_API_KEY", "NSH_API_KEY"],
            ProviderType::Gemini => &["GEMINI_API_KEY", "GOOGLE_API_KEY", "NSH_API_KEY"],
            ProviderType::OpenRouter => &["OPENROUTER_API_KEY", "NSH_API_KEY"],
            ProviderType::OpenAICompatible => &["OPENAI_API_KEY", "NSH_API_KEY"],
        }
    }
}

impl AiConfig {
    /// Trimmed API key from config, or provider-specific env vars as fallback.
    pub fn resolved_api_key(&self) -> Option<String> {
        if let Some(k) = self.api_key.as_ref() {
            let t = k.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
        for var in self.provider.api_key_env_vars() {
            if let Ok(v) = std::env::var(var) {
                let t = v.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
        }
        None
    }

    /// True when the key looks long enough to be a real credential (not a typo).
    pub fn has_usable_api_key(&self) -> bool {
        self.resolved_api_key()
            .map(|k| k.len() >= 8)
            .unwrap_or(false)
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
            ProviderType::Gemini => write!(f, "gemini"),
            ProviderType::OpenRouter => write!(f, "openrouter"),
            ProviderType::OpenAICompatible => write!(f, "openaicompatible"),
        }
    }
}
