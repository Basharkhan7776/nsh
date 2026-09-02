pub mod agent;
pub mod config;

pub use config::{AiConfig, ConfigError, ProviderType};

use aisdk::core::DynamicModel;
use aisdk::core::LanguageModelRequest;
use aisdk::providers::OpenAICompatible;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Config error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("AI not enabled")]
    NotEnabled,
    #[error("Request error: {0}")]
    Request(String),
    #[error("{0}")]
    MissingApiKey(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OllamaModel {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default, rename = "digest")]
    pub digest: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

pub async fn fetch_models(provider: ProviderType, base_url: &str) -> Vec<String> {
    match provider {
        ProviderType::Ollama => fetch_ollama_models(base_url)
            .await
            .unwrap_or_else(|_| default_ollama_models()),
        ProviderType::OpenAI => default_openai_models(),
        ProviderType::Anthropic => default_anthropic_models(),
        ProviderType::Gemini => default_gemini_models(),
        ProviderType::OpenRouter => default_openrouter_models(),
        ProviderType::OpenAICompatible => default_openai_models(),
    }
}

async fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));

    let response = client.get(&url).send().await?;
    let data: OllamaTagsResponse = response.json().await?;

    Ok(data.models.into_iter().map(|m| m.name).collect())
}

fn default_ollama_models() -> Vec<String> {
    vec![
        "llama3.2:latest".to_string(),
        "llama3.2:1b".to_string(),
        "llama3.2:7b".to_string(),
        "llama3.1:latest".to_string(),
        "llama3.1:8b".to_string(),
        "mistral:latest".to_string(),
        "codellama:latest".to_string(),
        "phi3:latest".to_string(),
    ]
}

fn default_openai_models() -> Vec<String> {
    vec![
        "gpt-4.1".to_string(),
        "gpt-4.1-mini".to_string(),
        "gpt-4.1-nano".to_string(),
        "gpt-4o".to_string(),
        "gpt-4o-mini".to_string(),
        "o3".to_string(),
        "o4-mini".to_string(),
        "gpt-4-turbo".to_string(),
        "gpt-3.5-turbo".to_string(),
    ]
}

fn default_anthropic_models() -> Vec<String> {
    vec![
        "claude-opus-4-20250514".to_string(),
        "claude-sonnet-4-20250514".to_string(),
        "claude-3-7-sonnet-20250219".to_string(),
        "claude-3-5-sonnet-20241022".to_string(),
        "claude-3-5-haiku-20241022".to_string(),
        "claude-3-opus-20240229".to_string(),
        "claude-3-haiku-20240307".to_string(),
    ]
}

fn default_gemini_models() -> Vec<String> {
    vec![
        // Gemini 3 series (latest)
        "gemini-3.5-flash".to_string(),
        "gemini-3.1-flash-lite".to_string(),
        "gemini-3.1-pro-preview".to_string(),
        "gemini-3-flash-preview".to_string(),
        // Gemini 2.5 series
        "gemini-2.5-pro".to_string(),
        "gemini-2.5-flash".to_string(),
        "gemini-2.5-flash-lite".to_string(),
        // Convenience aliases
        "gemini-flash-latest".to_string(),
        "gemini-pro-latest".to_string(),
    ]
}

fn default_openrouter_models() -> Vec<String> {
    vec![
        "openrouter/auto".to_string(),
        "stealth/ox-alpha".to_string(),
        "anthropic/claude-sonnet-4".to_string(),
        "anthropic/claude-opus-4".to_string(),
        "openai/gpt-4.1".to_string(),
        "openai/gpt-4o".to_string(),
        "openai/gpt-4o-mini".to_string(),
        "google/gemini-2.5-pro".to_string(),
        "google/gemini-2.5-flash".to_string(),
        "google/gemini-3.5-flash".to_string(),
        "meta-llama/llama-3.3-70b-instruct".to_string(),
        "deepseek/deepseek-r1".to_string(),
        "deepseek/deepseek-chat".to_string(),
        "qwen/qwen3-235b-a22b".to_string(),
        "mistralai/mistral-large".to_string(),
        "x-ai/grok-3".to_string(),
    ]
}

pub struct AiProvider {
    provider: OpenAICompatible<DynamicModel>,
    config: config::AiConfig,
}

impl AiProvider {
    pub fn new(config: config::AiConfig) -> Result<Self, AiError> {
        if config.provider.requires_api_key() && !config.has_usable_api_key() {
            let envs = config.provider.api_key_env_vars().join(", ");
            return Err(AiError::MissingApiKey(format!(
                "No valid API key for {}. Set it in Settings (API Key — paste with Ctrl+Shift+V), \
                 or export one of: {}. Current config key length: {}.",
                config.provider.display_name(),
                if envs.is_empty() {
                    "(none)".into()
                } else {
                    envs
                },
                config.api_key.as_ref().map(|k| k.trim().len()).unwrap_or(0),
            )));
        }

        let api_key = config.resolved_api_key().unwrap_or_default();
        // Normalize base URL: OpenAI-compatible clients often join paths poorly
        // if a trailing slash is missing/doubled.
        let base_url = config.base_url.trim().trim_end_matches('/').to_string();

        let provider = OpenAICompatible::builder()
            .model_name(config.model.clone())
            .base_url(base_url)
            .api_key(api_key)
            .build()
            .map_err(|e| AiError::Provider(format!("Failed to create provider: {e}")))?;

        Ok(Self { provider, config })
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn config(&self) -> &config::AiConfig {
        &self.config
    }

    pub async fn chat(&self, messages: Vec<String>) -> Result<String, AiError> {
        if !self.config.enabled {
            return Err(AiError::NotEnabled);
        }

        let user_message = messages.join("\n");

        let mut builder = LanguageModelRequest::builder()
            .model(self.provider.clone())
            .prompt(user_message);
        builder.max_output_tokens = Some(8192);

        let response = builder
            .build()
            .generate_text()
            .await
            .map_err(|e| AiError::Provider(e.to_string()))?;

        Ok(response.text().unwrap_or_default().to_string())
    }
}

pub fn create_provider(config: config::AiConfig) -> Result<AiProvider, AiError> {
    AiProvider::new(config)
}
