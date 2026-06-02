//! `patent` — a prior-art search for your code ideas.
//!
//! Takes a plain-English dev-tool idea and searches the open-source ecosystem —
//! crates.io, npm, PyPI, GitHub, Go, Maven, NuGet, RubyGems, Docker Hub, the VS
//! Code Marketplace, and Hacker News — for prior art, then gives an honest,
//! scoped verdict on whether it's already been built. The exact set searched is
//! chosen per query; whichever sources actually responded are always surfaced.
//!
//! **Integrity principle:** this tool can prove something *exists*, but never
//! that it *doesn't* — it only searched some sources. All output is scoped to
//! "what was found in the sources checked."
//!
//! # Install
//!
//! ```bash
//! cargo install patent
//! ```
//!
//! # Usage
//!
//! ```bash
//! patent "interactive cli to kill whatever's on a port"   # interactive TUI
//! patent "react component for infinite scroll" --json      # structured output
//! patent "kubernetes log viewer" --fast                    # skip the LLM verdict
//! ```
//!
//! # Using the library
//!
//! `patent` is primarily the engine behind the CLI of the same name, but the
//! core is reusable: [`sources::search_all`] fans out to the registries,
//! [`rank`] orders matches by semantic similarity, and [`verdict::assess`]
//! turns them into an integrity-scoped [`Verdict`] via a local Ollama model.

pub mod model;
pub mod ollama;
pub mod rank;
pub mod sources;
pub mod tui;
pub mod verdict;

pub use model::{Match, Query, Saturation, Source, Verdict};

/// Library-level error type. The binary maps these to `anyhow` with context.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("failed to parse response: {0}")]
    Parse(String),

    #[error("ollama not reachable at {0} — run `ollama serve` and `ollama pull qwen2.5`")]
    OllamaUnreachable(String),

    /// Ollama is reachable but rejected the request — most commonly because the
    /// requested model has not been pulled. Carries a human-readable reason.
    #[error("ollama could not generate a verdict: {0}")]
    OllamaModel(String),

    #[error("embedding failed: {0}")]
    Embedding(String),
}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, Error>;
