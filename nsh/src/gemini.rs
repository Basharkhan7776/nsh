use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;

pub struct GeminiClient {
    api_key: String,
    client: Client,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    embedding: EmbeddingValues,
}

#[derive(Deserialize)]
struct EmbeddingValues {
    values: Vec<f32>,
}

#[derive(Deserialize)]
struct BatchEmbeddingResponse {
    embeddings: Vec<EmbeddingValues>,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize, Serialize)]
pub struct Content {
    parts: Vec<Part>,
    role: String,
}

#[derive(Deserialize, Serialize)]
pub struct Part {
    text: String,
}

impl GeminiClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("GEMINI_API_KEY").context("GEMINI_API_KEY must be set")?;
        Ok(Self {
            api_key,
            client: Client::new(),
        })
    }

    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent?key={}",
            self.api_key
        );

        let payload = json!({
            "model": "models/text-embedding-004",
            "content": {
                "parts": [{
                    "text": text
                }]
            }
        });

        let resp = self.client.post(&url).json(&payload).send().await?;
        
        if !resp.status().is_success() {
             let error_text = resp.text().await?;
             anyhow::bail!("Gemini API Error: {}", error_text);
        }

        let data: EmbeddingResponse = resp.json().await?;
        Ok(data.embedding.values)
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            self.api_key
        );

        let payload = json!({
            "contents": [{
                "role": "user",
                "parts": [{ "text": prompt }]
            }]
        });

        let resp = self.client.post(&url).json(&payload).send().await?;
         if !resp.status().is_success() {
             let error_text = resp.text().await?;
             anyhow::bail!("Gemini API Error: {}", error_text);
        }

        let data: GenerateContentResponse = resp.json().await?;
        
        data.candidates.first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .context("No content generated")
    }
}
