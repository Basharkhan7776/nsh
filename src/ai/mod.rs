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
