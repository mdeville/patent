//! Hacker News source — `GET https://hn.algolia.com/api/v1/search?query=`
//! (free, no key).

use serde::Deserialize;

use super::Source;
use crate::model::{Match, Query, Source as SourceId};
use crate::Result;

const DEFAULT_BASE_URL: &str = "https://hn.algolia.com";

/// Searches Hacker News via the Algolia API.
#[derive(Debug, Clone)]
pub struct HackerNews {
    client: reqwest::Client,
    base_url: String,
}

impl HackerNews {
    /// Construct against the live HN Algolia API.
    pub fn new(client: reqwest::Client) -> Self {
        Self::with_base_url(client, DEFAULT_BASE_URL.to_string())
    }

    /// Construct against an arbitrary base URL (used by tests).
    pub fn with_base_url(client: reqwest::Client, base_url: String) -> Self {
        Self { client, base_url }
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
struct Hit {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    story_text: Option<String>,
    #[serde(rename = "objectID")]
    object_id: String,
    #[serde(default)]
    points: Option<u64>,
}

#[async_trait::async_trait]
impl Source for HackerNews {
    fn id(&self) -> SourceId {
        SourceId::HackerNews
    }

    async fn search(&self, query: &Query) -> Result<Vec<Match>> {
        let url = format!("{}/api/v1/search", self.base_url);
        let q = query.keywords.join(" ");

        let body: SearchResponse = self
            .client
            .get(&url)
            .query(&[("query", q.as_str())])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(body
            .hits
            .into_iter()
            .map(|h| Match {
                name: h.title.unwrap_or_default(),
                source: SourceId::HackerNews,
                // The canonical link is the HN discussion, not the (optional)
                // outbound URL — the discussion is always present.
                url: format!("https://news.ycombinator.com/item?id={}", h.object_id),
                description: h.story_text.unwrap_or_default(),
                popularity: h.points,
                similarity: 0.0,
            })
            .collect())
    }
}
