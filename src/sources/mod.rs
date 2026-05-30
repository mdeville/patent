//! Source registry: one implementor per ecosystem, fanned out concurrently.
//!
//! Each [`Source`] is independent and best-effort — a failing source is skipped
//! and reported as "not reached", never fatal to the run.

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

/// Fan out to every source concurrently. M2 fills this in (`join_all`, graceful
/// per-source failure, dedup).
pub async fn search_all(_query: &Query) -> Vec<Match> {
    todo!("M2: concurrent fan-out + dedup")
}
