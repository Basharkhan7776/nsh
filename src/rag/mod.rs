use crate::ai::AiConfig;
use crate::storage::{LocalStorage, VectorStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RagError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Vector error: {0}")]
    Vector(String),
    #[error("Embedding error: {0}")]
    Embedding(String),
    #[error("Not initialized")]
    NotInitialized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub source: String,
    pub metadata: Option<serde_json::Value>,
}

pub struct RagEngine {
    vector_store: VectorStore,
    embed_model: String,
    embed_base_url: String, // resolved, e.g. http://localhost:11434
}

impl RagEngine {
    /// Preferred constructor: derives embed base from AiConfig (for Ollama) + rag config.
    pub async fn new_from_config(
        storage: &LocalStorage,
        ai_config: &AiConfig,
        embed_model: Option<String>,
    ) -> Result<Self, RagError> {
        let rag_dir = storage.config_dir();
        let model = embed_model
            .or_else(|| ai_config.model.clone().into())
            .unwrap_or_else(|| "nomic-embed-text".to_string()); // sensible default if not set

        // Derive base for embeddings. Prefer ai base_url for Ollama-family.
        let base = match ai_config.provider {
            crate::ai::ProviderType::Ollama | crate::ai::ProviderType::OpenAICompatible => {
                // strip trailing /v1 if present
                ai_config.base_url.trim_end_matches('/').trim_end_matches("/v1").to_string()
            }
            _ => "http://localhost:11434".to_string(),
        };

        let vector_store = VectorStore::from_embedded(rag_dir, "nsh_rag")
            .await
            .map_err(|e| RagError::Vector(e.to_string()))?;

        Ok(Self {
            vector_store,
            embed_model: model,
            embed_base_url: base,
        })
    }

    /// Back-compat simple constructor (uses defaults).
    pub async fn new(storage: &LocalStorage, embed_model: &str) -> Result<Self, RagError> {
        let default_ai = AiConfig::default();
        Self::new_from_config(storage, &default_ai, Some(embed_model.to_string())).await
    }

    fn resolve_embed_url(&self) -> String {
        format!("{}/api/embeddings", self.embed_base_url.trim_end_matches('/'))
    }

    pub async fn index_document(&mut self, doc: Document) -> Result<(), RagError> {
        let vector = self.embed_text(&doc.content).await?;

        let payload: HashMap<String, serde_json::Value> = serde_json::from_value(serde_json::json!({
            "id": doc.id,
            "content": doc.content,
            "source": doc.source,
            "metadata": doc.metadata,
        })).map_err(|e| RagError::Vector(e.to_string()))?;

        self.vector_store
            .add_points(vec![vector], vec![payload])
            .await
            .map_err(|e| RagError::Vector(e.to_string()))?;

        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<RetrievedDocument>, RagError> {
        let query_vector = self.embed_text(query).await?;

        let results = self.vector_store
            .search(query_vector, limit)
            .await
            .map_err(|e| RagError::Vector(e.to_string()))?;

        Ok(results
            .into_iter()
            .filter_map(|r| {
                let content = r.payload.get("content")?.to_string();
                let source = r.payload.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let id = r.payload.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                
                Some(RetrievedDocument {
                    id,
                    content: content.trim_matches('"').to_string(), // clean serde string form
                    source,
                    score: r.score,
                })
            })
            .collect())
    }

    /// Convenience for agent: returns a compact context string.
    pub async fn retrieve_context(&self, query: &str, k: usize) -> String {
        match self.search(query, k).await {
            Ok(docs) if !docs.is_empty() => {
                let mut out = String::from("Relevant context from your indexed documents:\n");
                for (i, d) in docs.iter().enumerate() {
                    out.push_str(&format!("---\n[{}]\n{}\n", d.source, d.content));
                    if i >= 2 { break; }
                }
                out
            }
            _ => String::new(),
        }
    }

    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, RagError> {
        let client = reqwest::Client::new();
        let url = self.resolve_embed_url();

        let response = client
            .post(&url)
            .json(&serde_json::json!({
                "model": self.embed_model,
                "prompt": text
            }))
            .send()
            .await
            .map_err(|e| RagError::Embedding(e.to_string()))?;

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RagError::Embedding(e.to_string()))?;

        let embedding = data["embedding"]
            .as_array()
            .ok_or_else(|| RagError::Embedding("Invalid embedding response".into()))?
            .iter()
            .filter_map(|v| v.as_f64())
            .map(|v| v as f32)
            .collect();

        Ok(embedding)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    pub id: String,
    pub content: String,
    pub source: String,
    pub score: f32,
}
