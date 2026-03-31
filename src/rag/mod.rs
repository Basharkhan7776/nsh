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
}

impl RagEngine {
    pub async fn new(storage: &LocalStorage, embed_model: &str) -> Result<Self, RagError> {
        let vector_store = VectorStore::from_embedded(
            storage.config_dir(),
            "nsh_rag",
        )
        .await
        .map_err(|e| RagError::Vector(e.to_string()))?;

        Ok(Self {
            vector_store,
            embed_model: embed_model.to_string(),
        })
    }

    pub async fn index_document(&self, doc: Document) -> Result<(), RagError> {
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
                    content,
                    source,
                    score: r.score,
                })
            })
            .collect())
    }

    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, RagError> {
        let client = reqwest::Client::new();
        
        let response = client
            .post(format!("http://localhost:11434/api/embeddings"))
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
