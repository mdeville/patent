//! Source registry: one implementor per ecosystem, fanned out concurrently.
//!
//! Sources are selected based on the query — a Rust query searches crates.io,
//! a Python query searches PyPI, etc. GitHub is always included. When no
//! language is detected, the three largest registries (npm, PyPI, crates.io)
//! are used as a broad fallback.

use std::collections::HashSet;
use std::time::Duration;

use futures::future::join_all;

use crate::model::{Match, Query};
use crate::Result;

pub mod crates_io;
pub mod docker_hub;
pub mod github;
pub mod go;
pub mod hacker_news;
pub mod maven;
pub mod npm;
pub mod nuget;
pub mod pypi;
pub mod rubygems;
pub mod vscode;

/// One searchable ecosystem (a registry, a forge, a community index).
#[async_trait::async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Stable identifier, used in the transparency line.
    fn id(&self) -> crate::model::Source;

    /// Search this source for prior art matching `query`.
    async fn search(&self, query: &Query) -> Result<Vec<Match>>;
}

use crate::model::Source as S;

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(5))
        .user_agent(concat!(
            "patent/",
            env!("CARGO_PKG_VERSION"),
            " (prior-art search)"
        ))
        .build()
        .expect("failed to build HTTP client")
}

fn idea_contains(idea: &str, terms: &[&str]) -> bool {
    let lower = idea.to_lowercase();
    terms.iter().any(|t| {
        lower.find(t).is_some_and(|pos| {
            let before = pos == 0 || !lower.as_bytes()[pos - 1].is_ascii_alphanumeric();
            let after_pos = pos + t.len();
            let after =
                after_pos >= lower.len() || !lower.as_bytes()[after_pos].is_ascii_alphanumeric();
            before && after
        })
    })
}

fn add(set: &mut HashSet<S>, sources: &[S]) {
    set.extend(sources);
}

fn detect_sources(idea: &str) -> HashSet<S> {
    let mut s = HashSet::new();

    // GitHub is always included.
    s.insert(S::GitHub);

    // ── Explicit language / ecosystem mentions ──────────────────────────
    if idea_contains(idea, &["rust", "crate", "cargo"]) {
        s.insert(S::CratesIo);
    }
    if idea_contains(
        idea,
        &["npm", "node", "javascript", "typescript", "deno", "bun"],
    ) {
        s.insert(S::Npm);
    }
    if idea_contains(
        idea,
        &["python", "pip", "django", "flask", "pytorch", "pandas"],
    ) {
        s.insert(S::PyPI);
    }
    if idea_contains(idea, &["go ", "golang", "goroutine"]) {
        s.insert(S::Go);
    }
    if idea_contains(
        idea,
        &["java", "kotlin", "spring", "maven", "gradle", "scala"],
    ) {
        s.insert(S::Maven);
    }
    if idea_contains(idea, &["ruby", "rails", "sinatra", "gem"]) {
        s.insert(S::RubyGems);
    }
    if idea_contains(
        idea,
        &["c#", ".net", "csharp", "dotnet", "nuget", "blazor", "unity"],
    ) {
        s.insert(S::NuGet);
    }

    // ── Domain inference (no language named, but the problem implies one) ─
    if idea_contains(
        idea,
        &[
            "ai ",
            "llm",
            "machine learning",
            "deep learning",
            "neural",
            "model training",
            "inference",
            "embedding",
            "nlp",
            "computer vision",
            "data science",
            "data pipeline",
        ],
    ) {
        add(&mut s, &[S::PyPI, S::Npm]);
    }
    if idea_contains(idea, &["cli", "command line", "terminal tool", "shell"]) {
        add(&mut s, &[S::CratesIo, S::Go]);
    }
    if idea_contains(
        idea,
        &[
            "frontend",
            "react",
            "vue",
            "angular",
            "svelte",
            "browser",
            "css",
            "ui component",
            "web component",
            "spa",
        ],
    ) {
        s.insert(S::Npm);
    }
    if idea_contains(
        idea,
        &[
            "api",
            "backend",
            "rest",
            "graphql",
            "microservice",
            "web server",
        ],
    ) {
        add(&mut s, &[S::Npm, S::PyPI, S::Go]);
    }
    if idea_contains(
        idea,
        &[
            "mobile",
            "ios",
            "android",
            "react native",
            "flutter",
            "swift",
            "swiftui",
        ],
    ) {
        add(&mut s, &[S::Npm, S::Maven]);
    }
    if idea_contains(
        idea,
        &[
            "game",
            "graphics",
            "rendering",
            "opengl",
            "vulkan",
            "bevy",
            "godot",
        ],
    ) {
        add(&mut s, &[S::CratesIo, S::NuGet]);
    }
    if idea_contains(idea, &["embedded", "firmware", "microcontroller", "rtos"]) {
        s.insert(S::CratesIo);
    }
    if idea_contains(
        idea,
        &[
            "docker",
            "container",
            "kubernetes",
            "k8s",
            "helm",
            "deploy",
            "infrastructure",
        ],
    ) {
        add(&mut s, &[S::DockerHub, S::Go]);
    }
    if idea_contains(idea, &["vscode", "extension", "plugin", "ide", "editor"]) {
        add(&mut s, &[S::VsCodeMarketplace, S::Npm]);
    }

    // ── Fallback: no signal at all → broad sweep ────────────────────────
    // GitHub alone isn't enough; add the 3 biggest registries.
    if s.len() <= 1 {
        add(&mut s, &[S::Npm, S::PyPI, S::CratesIo]);
    }

    s
}

fn build_source(id: S, client: reqwest::Client) -> Box<dyn SourceAdapter> {
    match id {
        S::CratesIo => Box::new(crates_io::CratesIo::new(client)),
        S::GitHub => Box::new(github::GitHub::new(client)),
        S::Npm => Box::new(npm::Npm::new(client)),
        S::PyPI => Box::new(pypi::PyPI::new(client)),
        S::HackerNews => Box::new(hacker_news::HackerNews::new(client)),
        S::Go => Box::new(go::GoPkgDev::new(client)),
        S::Maven => Box::new(maven::Maven::new(client)),
        S::RubyGems => Box::new(rubygems::RubyGems::new(client)),
        S::DockerHub => Box::new(docker_hub::DockerHub::new(client)),
        S::VsCodeMarketplace => Box::new(vscode::VsCodeMarketplace::new(client)),
        S::NuGet => Box::new(nuget::NuGet::new(client)),
    }
}

/// Pick sources based on what the query is about.
fn sources_for(query: &Query) -> Vec<Box<dyn SourceAdapter>> {
    let client = http_client();
    let ids = detect_sources(&query.idea);
    ids.into_iter()
        .map(|id| build_source(id, client.clone()))
        .collect()
}

/// Fan out to selected sources concurrently, dropping the ones that fail.
/// Returns the deduped matches and which sources were reached.
pub async fn search_all(query: &Query) -> (Vec<Match>, Vec<crate::model::Source>) {
    search_sources(&sources_for(query), query).await
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
        async move {
            let first = s.search(query).await;
            if first.is_ok() {
                return (id, first);
            }
            tokio::time::sleep(Duration::from_millis(800)).await;
            (id, s.search(query).await)
        }
    }))
    .await;

    let mut reached = Vec::new();
    let mut failed = Vec::new();
    let mut all = Vec::new();
    for (id, result) in results {
        match result {
            Ok(matches) => {
                reached.push(id);
                all.extend(matches);
            }
            Err(e) => {
                failed.push((id, e));
            }
        }
    }
    for (id, err) in &failed {
        eprintln!("⚠  {id} failed: {err}");
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
