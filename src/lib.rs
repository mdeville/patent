//! `patent` — a prior-art search for your code ideas.
//!
//! Takes a plain-English dev-tool idea and searches the open-source ecosystem
//! (crates.io, GitHub, npm, PyPI, Hacker News) for prior art, then gives an
//! honest, scoped verdict on whether it's already been built.
//!
//! **Integrity principle:** this tool can prove something *exists*, but never
//! that it *doesn't* — it only searched some sources. All output is scoped to
//! "what was found in the sources checked."

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

    #[error("embedding failed: {0}")]
    Embedding(String),
}

/// Crate result alias.
pub type Result<T> = std::result::Result<T, Error>;
