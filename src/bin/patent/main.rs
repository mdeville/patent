//! `patent` binary — thin CLI/TUI shell over the `patent` library.

mod cli;
mod tui;

use clap::Parser;
use cli::Cli;

/// Matches below this similarity are noise, not signal.
const MIN_RELEVANCE: f32 = 0.35;

fn validate_idea(idea: &str) -> anyhow::Result<()> {
    let trimmed = idea.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Please provide a dev-tool idea to search for.");
    }

    let meaningful: Vec<String> = trimmed
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_ascii_alphanumeric()) && w.len() > 2)
        .map(|w| w.to_lowercase())
        .collect();

    if meaningful.len() < 3 {
        anyhow::bail!(
            "Too vague — describe a specific software tool or feature, e.g.\n  \
             patent \"CLI tool that kills a process on a given port\""
        );
    }

    let unique: std::collections::HashSet<&str> = meaningful.iter().map(|w| w.as_str()).collect();
    if unique.len() < 3 {
        anyhow::bail!(
            "Too repetitive — describe what the tool does, e.g.\n  \
             patent \"CLI tool that kills a process on a given port\""
        );
    }

    Ok(())
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one",
    "our", "out", "has", "its", "let", "may", "who", "did", "get", "got", "how", "his", "him",
    "she", "also", "been", "call", "each", "from", "have", "into", "just", "like", "long", "make",
    "many", "more", "most", "much", "must", "name", "only", "over", "some", "such", "than", "that",
    "them", "then", "they", "this", "very", "when", "what", "with", "will", "your", "which",
    "about", "after", "being", "could", "every", "first", "found", "great", "where", "these",
    "their", "there", "those", "would", "other", "should", "before", "between", "best", "near",
    "here", "well", "does", "were",
];

fn strip_punctuation(word: &str) -> String {
    word.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

fn build_query(idea: &str) -> patent::Query {
    let keywords: Vec<String> = idea
        .split_whitespace()
        .map(|w| strip_punctuation(&w.to_lowercase()))
        .filter(|w| w.len() > 2 && !STOPWORDS.contains(&w.as_str()))
        .collect();
    patent::Query {
        idea: idea.to_string(),
        keywords,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    validate_idea(&args.idea)?;

    let query = build_query(&args.idea);
    eprintln!("🔍 Searching for prior art: \"{}\"", args.idea);
    eprintln!("   keywords: {}", query.keywords.join(", "));

    // ── Phase 1: search sources AND load embedding model concurrently ───
    // Model init is CPU-bound (~1-3 s), searches are I/O-bound (~2-5 s).
    // Overlapping them saves the entire model-load latency.
    let t_start = std::time::Instant::now();
    let idea_for_embed = query.idea.clone();
    let (search_result, ranker_result) = tokio::join!(
        patent::sources::search_all(&query),
        tokio::task::spawn_blocking(move || {
            let ranker = patent::rank::Ranker::new()?;
            let query_emb = ranker.embed_query(&idea_for_embed)?;
            Ok::<_, patent::Error>((ranker, query_emb))
        })
    );

    let (raw_matches, reached) = search_result;
    let (ranker, query_emb) = ranker_result.expect("embedding task panicked")?;

    eprintln!(
        "   {} matches from {} sources in {:.1}s: {}",
        raw_matches.len(),
        reached.len(),
        t_start.elapsed().as_secs_f64(),
        reached
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // ── Phase 2: rank (embed descriptions + cosine sort) ────────────────
    let t_rank = std::time::Instant::now();
    let limit = args.limit;
    let ranked =
        tokio::task::spawn_blocking(move || ranker.rank_with(&query_emb, raw_matches, limit))
            .await
            .expect("ranking task panicked")?;
    eprintln!(
        "📊 Ranked to top {} in {:.1}s",
        ranked.len(),
        t_rank.elapsed().as_secs_f64(),
    );

    // ── Phase 3: relevance gate + verdict ───────────────────────────────
    let best_sim = ranked.first().map_or(0.0, |m| m.similarity);
    let verdict = if best_sim < MIN_RELEVANCE {
        eprintln!(
            "⚠  Best similarity {:.2} < {:.2} — skipping verdict",
            best_sim, MIN_RELEVANCE,
        );
        patent::Verdict {
            level: patent::Saturation::Open,
            headline: "Nothing relevant turned up in the sources checked. \
                       The query may not describe a recognized software tool — \
                       try rephrasing with specific technical terms."
                .into(),
            gaps: vec![],
            sources_checked: reached,
            caveat: patent::verdict::CAVEAT.to_string(),
        }
    } else {
        let t_verdict = std::time::Instant::now();
        eprintln!("🧠 Generating verdict via Ollama ({})…", args.model);
        let ollama = patent::ollama::Ollama::new(patent::ollama::DEFAULT_ENDPOINT, &args.model);
        let v = patent::verdict::assess(&ollama, &query, &ranked, reached).await?;
        eprintln!("   verdict in {:.1}s", t_verdict.elapsed().as_secs_f64());
        v
    };

    eprintln!("⏱  total: {:.1}s", t_start.elapsed().as_secs_f64());

    // ── Phase 4: output ─────────────────────────────────────────────────
    if args.json {
        let output = serde_json::json!({
            "query": args.idea,
            "verdict": verdict,
            "matches": ranked,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        tui::run(&args.idea, &verdict, &ranked)?;
    }

    Ok(())
}
