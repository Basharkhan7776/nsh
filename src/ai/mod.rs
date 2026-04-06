pub mod config;

pub use config::{AiConfig, ConfigError, ProviderType};

use aisdk::providers::OpenAICompatible;
use aisdk::core::DynamicModel;
use aisdk::core::LanguageModelRequest;
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
        ProviderType::Ollama => {
            fetch_ollama_models(base_url).await.unwrap_or_else(|_| default_ollama_models())
        }
        ProviderType::OpenAI => default_openai_models(),
        ProviderType::Anthropic => default_anthropic_models(),
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
        "gpt-4o".to_string(),
        "gpt-4o-mini".to_string(),
        "gpt-4-turbo".to_string(),
        "gpt-3.5-turbo".to_string(),
    ]
}

fn default_anthropic_models() -> Vec<String> {
    vec![
        "claude-sonnet-4-20250514".to_string(),
        "claude-3-5-sonnet-20241022".to_string(),
        "claude-3-5-haiku-20241022".to_string(),
        "claude-3-opus-20240229".to_string(),
        "claude-3-haiku-20240307".to_string(),
    ]
}

pub struct AiProvider {
    provider: OpenAICompatible<DynamicModel>,
    config: config::AiConfig,
}

impl AiProvider {
    pub fn new(config: config::AiConfig) -> Self {
        let provider = OpenAICompatible::builder()
            .model_name(config.model.clone())
            .base_url(config.base_url.clone())
            .api_key(config.api_key.clone().unwrap_or_default())
            .build()
            .expect("Failed to create OpenAICompatible provider");
        
        Self { provider, config }
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
        
        let response = LanguageModelRequest::builder()
            .model(self.provider.clone())
            .prompt(user_message)
            .build()
            .generate_text()
            .await
            .map_err(|e| AiError::Provider(e.to_string()))?;

        Ok(response.text().unwrap_or_default().to_string())
    }
}

pub fn create_provider(config: config::AiConfig) -> AiProvider {
    AiProvider::new(config)
}
