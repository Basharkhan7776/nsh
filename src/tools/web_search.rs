use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoResponse {
    #[serde(rename = "AbstractText")]
    abstract_text: Option<String>,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
    #[serde(rename = "RelatedTopics")]
    related_topics: Option<Vec<RelatedTopic>>,
}

#[derive(Debug, Deserialize)]
struct RelatedTopic {
    #[serde(alias = "Text")]
    text: Option<String>,
    #[serde(alias = "FirstURL")]
    first_url: Option<String>,
}

pub async fn web_search(query: &str) -> Result<Vec<SearchResult>, ToolError> {
    let client = Client::new();
    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        urlencoding::encode(query)
    );

    let response = client.get(&url).send().await?;
    let data: DuckDuckGoResponse = response.json().await?;

    let mut results = Vec::new();

    if let Some(text) = data.abstract_text {
        if !text.is_empty() {
            results.push(SearchResult {
                title: query.to_string(),
                url: data.abstract_url.unwrap_or_default(),
                snippet: text,
            });
        }
    }

    if let Some(topics) = data.related_topics {
        for topic in topics.into_iter().take(5) {
            if let Some(text) = topic.text {
                results.push(SearchResult {
                    title: text.split(' ').take(5).collect::<Vec<_>>().join(" "),
                    url: topic.first_url.unwrap_or_default(),
                    snippet: text,
                });
            }
        }
    }

    Ok(results)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

impl std::fmt::Display for SearchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}\n{}", self.title, self.url, self.snippet)
    }
}
