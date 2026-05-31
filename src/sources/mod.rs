//! Source registry: one implementor per ecosystem, fanned out concurrently.
//!
//! Each [`Source`] is independent and best-effort — a failing source is skipped
//! and reported as "not reached", never fatal to the run.

use std::collections::HashSet;

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
pub trait Source: Send + Sync {
    /// Stable identifier, used in the transparency line.
    fn id(&self) -> crate::model::Source;

    /// Search this source for prior art matching `query`.
    async fn search(&self, query: &Query) -> Result<Vec<Match>>;
}

/// Every source, sharing one HTTP client.
fn default_sources() -> Vec<Box<dyn Source>> {
    let client = reqwest::Client::new();
    vec![
        Box::new(crates_io::CratesIo::new(client.clone())),
        Box::new(github::GitHub::new(client.clone())),
        Box::new(npm::Npm::new(client.clone())),
        Box::new(pypi::PyPI::new(client.clone())),
        Box::new(hacker_news::HackerNews::new(client)),
    ]
}

/// Fan out to every source concurrently, dropping the ones that fail.
pub async fn search_all(query: &Query) -> Vec<Match> {
    search_sources(&default_sources(), query).await
}

/// Run `query` against `sources` concurrently, skipping any that error, and
/// dedup the combined results. Exposed for testing the fan-out in isolation.
pub async fn search_sources(sources: &[Box<dyn Source>], query: &Query) -> Vec<Match> {
    let results = join_all(sources.iter().map(|s| s.search(query))).await;

    // Best-effort: `flatten` drops the `Err` results, so a failing source is
    // skipped, never fatal to the run.
    let mut all = Vec::new();
    for matches in results.into_iter().flatten() {
        all.extend(matches);
    }
    dedup(all)
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
