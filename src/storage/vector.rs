use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VectorError {
    #[error("Qdrant error: {0}")]
    Qdrant(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not initialized")]
    NotInitialized,
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct StoredPoint {
    vector: Vec<f32>,
    payload: HashMap<String, serde_json::Value>,
}

pub struct VectorStore {
    collection_name: String,
    points: Vec<StoredPoint>,
    persist_path: PathBuf,
}

impl VectorStore {
    pub async fn new<P: AsRef<Path>>(_storage_path: P, collection_name: &str) -> Result<Self, VectorError> {
        // Legacy constructor kept for compatibility; prefer from_embedded.
        let dir = _storage_path.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        let persist_path = dir.join(format!("{}.json", collection_name));
        let points = Self::load_points(&persist_path)?;
        Ok(Self {
            collection_name: collection_name.to_string(),
            points,
            persist_path,
        })
    }

    pub async fn from_embedded<P: AsRef<Path>>(storage_path: P, collection_name: &str) -> Result<Self, VectorError> {
        let dir = storage_path.as_ref().join("rag");
        std::fs::create_dir_all(&dir)?;
        let persist_path = dir.join(format!("{}.json", collection_name));
        let points = Self::load_points(&persist_path)?;
        Ok(Self {
            collection_name: collection_name.to_string(),
            points,
            persist_path,
        })
    }

    fn load_points(path: &Path) -> Result<Vec<StoredPoint>, VectorError> {
        if !path.exists() {
            return Ok(vec![]);
        }
        let data = std::fs::read_to_string(path)?;
        if data.trim().is_empty() {
            return Ok(vec![]);
        }
        let points: Vec<StoredPoint> = serde_json::from_str(&data)?;
        Ok(points)
    }

    fn save_points(&self) -> Result<(), VectorError> {
        let data = serde_json::to_string_pretty(&self.points)?;
        std::fs::write(&self.persist_path, data)?;
        Ok(())
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let mut dot = 0.0f32;
        let mut na = 0.0f32;
        let mut nb = 0.0f32;
        for (x, y) in a.iter().zip(b.iter()) {
            dot += x * y;
            na += x * x;
            nb += y * y;
        }
        let denom = (na.sqrt() * nb.sqrt()).max(1e-8);
        (dot / denom).clamp(-1.0, 1.0)
    }

    pub async fn add_points(
        &mut self,
        vectors: Vec<Vec<f32>>,
        payloads: Vec<HashMap<String, serde_json::Value>>,
    ) -> Result<(), VectorError> {
        for (vec, payload) in vectors.into_iter().zip(payloads.into_iter()) {
            self.points.push(StoredPoint { vector: vec, payload });
        }
        self.save_points()?;
        Ok(())
    }

    pub async fn search(&self, query_vector: Vec<f32>, limit: usize) -> Result<Vec<SearchResult>, VectorError> {
        let mut scored: Vec<(f32, usize)> = self
            .points
            .iter()
            .enumerate()
            .map(|(i, p)| (Self::cosine(&query_vector, &p.vector), i))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results = scored
            .into_iter()
            .take(limit)
            .map(|(score, idx)| {
                let p = &self.points[idx];
                SearchResult {
                    id: idx as i64,
                    score,
                    payload: p.payload.clone(),
                }
            })
            .collect();

        Ok(results)
    }

    pub fn collection_name(&self) -> &str {
        &self.collection_name
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: i64,
    pub score: f32,
    pub payload: HashMap<String, serde_json::Value>,
}
