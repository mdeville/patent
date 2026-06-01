//! GitHub source — `GET /search/repositories`. Reads optional `GITHUB_TOKEN`
//! from the environment to raise the unauthenticated rate limit.

use serde::Deserialize;

use super::SourceAdapter;
use crate::model::{Match, Query, Source};
use crate::Result;

const DEFAULT_BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = concat!("patent/", env!("CARGO_PKG_VERSION"), " (prior-art search)");

/// Searches GitHub repositories.
#[derive(Debug, Clone)]
pub struct GitHub {
    client: reqwest::Client,
    base_url: String,
    /// Personal access token, if `GITHUB_TOKEN` was set. Lifts the rate limit.
    token: Option<String>,
}

impl GitHub {
    /// Construct against the live GitHub API, reading `GITHUB_TOKEN` if present.
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            base_url: DEFAULT_BASE_URL.to_string(),
            token: std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()),
        }
    }

    /// Construct against an arbitrary base URL (used by tests). No token.
    pub fn with_base_url(client: reqwest::Client, base_url: String) -> Self {
        Self {
            client,
            base_url,
            token: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    items: Vec<Repo>,
}

#[derive(Debug, Deserialize)]
struct Repo {
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    html_url: String,
    #[serde(default)]
    stargazers_count: Option<u64>,
}

#[async_trait::async_trait]
impl SourceAdapter for GitHub {
    fn id(&self) -> Source {
        Source::GitHub
    }

    async fn search(&self, query: &Query) -> Result<Vec<Match>> {
        let url = format!("{}/search/repositories", self.base_url);
        let q = query.keywords.join(" ");

        let mut request = self
            .client
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .query(&[("q", q.as_str()), ("per_page", "20")]);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::FORBIDDEN && self.token.is_none() {
            return Err(crate::Error::Parse(
                "GitHub API rate limit exceeded — set GITHUB_TOKEN env var for higher limits"
                    .into(),
            ));
        }
        let body: SearchResponse = response.error_for_status()?.json().await?;

        Ok(body
            .items
            .into_iter()
            .filter(|r| r.description.as_ref().is_some_and(|d| !d.is_empty()))
            .map(|r| Match {
                name: r.full_name,
                source: Source::GitHub,
                url: r.html_url,
                description: r.description.unwrap_or_default(),
                popularity: r.stargazers_count,
                similarity: 0.0,
            })
            .collect())
    }
}
