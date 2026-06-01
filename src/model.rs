//! Core domain types shared across the pipeline.
//!
//! These are deliberately small and `serde`-serializable so the `--json` path
//! and the TUI render from the same data.

use serde::{Deserialize, Serialize};

/// A user's idea, plus keywords derived from it for keyword-based source APIs.
///
/// The full `idea` string is what the embedder ranks against; `keywords` is the
/// cleaned-up query handed to registry search endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub idea: String,
    pub keywords: Vec<String>,
}

/// Where a [`Match`] came from. Always surfaced to the user for transparency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Source {
    CratesIo,
    GitHub,
    Npm,
    PyPI,
    HackerNews,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CratesIo => f.write_str("crates.io"),
            Self::GitHub => f.write_str("GitHub"),
            Self::Npm => f.write_str("npm"),
            Self::PyPI => f.write_str("PyPI"),
            Self::HackerNews => f.write_str("Hacker News"),
        }
    }
}

/// A single piece of prior art found in a [`Source`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Match {
    pub name: String,
    pub source: Source,
    pub url: String,
    pub description: String,
    /// Source-specific popularity signal (downloads, stars, points…). Optional
    /// because not every source exposes one.
    pub popularity: Option<u64>,
    /// Cosine similarity to the idea, in `[0.0, 1.0]`. Filled in by `rank`.
    pub similarity: f32,
}

/// How crowded the space looks, based on what was found in the sources checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Saturation {
    /// 🟢 nothing close found in the sources checked.
    Open,
    /// 🟡 a few adjacent things exist.
    Crowded,
    /// 🔴 the space is densely populated.
    Saturated,
}

/// The model-written, integrity-scoped verdict.
///
/// Invariant: copy is always phrased as "found in the sources checked" and never
/// asserts that something does not exist anywhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    pub level: Saturation,
    pub headline: String,
    pub gaps: Vec<String>,
    pub sources_checked: Vec<Source>,
    pub caveat: String,
}
