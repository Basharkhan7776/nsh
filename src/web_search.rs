use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::env;

pub struct GoogleSearchClient {
    api_key: String,
    cse_id: String,
    client: Client,
}

#[derive(Deserialize)]
struct SearchResponse {
    items: Option<Vec<SearchResult>>,
}

#[derive(Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub link: String,
}

impl GoogleSearchClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("GOOGLE_API_KEY").context("GOOGLE_API_KEY must be set")?;
        let cse_id = env::var("GOOGLE_CSE_ID").context("GOOGLE_CSE_ID must be set")?;
        Ok(Self {
            api_key,
            cse_id,
            client: Client::new(),
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = "https://www.googleapis.com/customsearch/v1";
        
        let resp = self.client.get(url)
            .query(&[
                ("key", &self.api_key),
                ("cx", &self.cse_id),
                ("q", &query.to_string()),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            let txt = resp.text().await?;
            anyhow::bail!("Google Search API Error: {}", txt);
        }

        let data: SearchResponse = resp.json().await?;
        Ok(data.items.unwrap_or_default())
    }
}
