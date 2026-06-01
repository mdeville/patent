//! Source registry: one implementor per ecosystem, fanned out concurrently.
//!
//! Each source is independent and best-effort — a failing source is skipped
//! and reported as "not reached", never fatal to the run.

use std::collections::HashSet;
use std::time::Duration;

use futures::future::join_all;

use crate::model::{Match, Query};
use crate::Result;

pub mod crates_io;
pub mod github;
pub mod hacker_news;
pub mod npm;
pub mod pypi;

/// One searchable ecosystem (a registry, a forge, a community index).
#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Stable identifier, used in the transparency line.
    fn id(&self) -> crate::model::Source;

    /// Search this source for prior art matching `query`.
    async fn search(&self, query: &Query) -> Result<Vec<Match>>;
}

/// Every source, sharing one HTTP client with timeouts.
fn default_sources() -> Vec<Box<dyn SourceAdapter>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .user_agent(concat!(
            "patent/",
            env!("CARGO_PKG_VERSION"),
            " (prior-art search)"
        ))
        .build()
        .expect("failed to build HTTP client");
    vec![
        Box::new(crates_io::CratesIo::new(client.clone())),
        Box::new(github::GitHub::new(client.clone())),
        Box::new(npm::Npm::new(client.clone())),
        Box::new(pypi::PyPI::new(client.clone())),
        Box::new(hacker_news::HackerNews::new(client)),
    ]
}

/// Fan out to every source concurrently, dropping the ones that fail.
/// Returns the deduped matches and which sources were reached.
pub async fn search_all(query: &Query) -> (Vec<Match>, Vec<crate::model::Source>) {
    search_sources(&default_sources(), query).await
}

/// Run `query` against `sources` concurrently, skipping any that error, and
/// dedup the combined results. Returns the deduped matches and which sources
/// responded successfully. Exposed for testing the fan-out in isolation.
pub async fn search_sources(
    sources: &[Box<dyn SourceAdapter>],
    query: &Query,
) -> (Vec<Match>, Vec<crate::model::Source>) {
    let results = join_all(sources.iter().map(|s| {
        let id = s.id();
        async move { (id, s.search(query).await) }
    }))
    .await;

    let mut reached = Vec::new();
    let mut all = Vec::new();
    for (id, result) in results {
        if let Ok(matches) = result {
            reached.push(id);
            all.extend(matches);
        }
    }
    (dedup(all), reached)
}

/// Remove duplicate matches by URL, keeping the first occurrence and preserving
/// order. URL is a match's canonical identity across sources.
pub fn dedup(matches: Vec<Match>) -> Vec<Match> {
    let mut seen = HashSet::new();
    matches
        .into_iter()
        .filter(|m| seen.insert(m.url.clone()))
        .collect()
}
