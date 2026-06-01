//! `patent` binary — thin CLI/TUI shell over the `patent` library.

mod cli;
mod tui;

use clap::{CommandFactory, Parser};
use cli::Cli;
use std::io::IsTerminal;

/// Matches below this similarity are noise, not signal.
const MIN_RELEVANCE: f32 = 0.35;

fn validate_idea(idea: &str) -> anyhow::Result<()> {
    let trimmed = idea.trim();
    if trimmed.is_empty() {
        anyhow::bail!("Please provide a dev-tool idea to search for.");
    }

    // Unicode-aware: a "word" is a whitespace-delimited token with at least
    // three alphanumeric characters in any script (not just ASCII).
    let meaningful: Vec<String> = trimmed
        .split_whitespace()
        .filter(|w| w.chars().filter(|c| c.is_alphanumeric()).count() >= 3)
        .map(|w| w.to_lowercase())
        .collect();

    // Scripts that don't separate words with spaces (e.g. Chinese, Japanese)
    // can't be tokenized into words this way. Accept them based on a count of
    // non-ASCII alphanumeric characters — gating on non-ASCII so this doesn't
    // become an escape hatch that lets short ASCII gibberish skip the checks
    // below.
    let space_delimited = trimmed.split_whitespace().count() >= 3;
    let cjk_like = trimmed
        .chars()
        .filter(|c| c.is_alphanumeric() && !c.is_ascii())
        .count();
    if !space_delimited && cjk_like >= 2 {
        return Ok(());
    }

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

    if let Some(shell) = args.completions {
        clap_complete::generate(shell, &mut Cli::command(), "patent", &mut std::io::stdout());
        return Ok(());
    }

    let idea = args
        .idea
        .expect("idea is required when not using --completions");
    validate_idea(&idea)?;

    let query = build_query(&idea);
    eprintln!("🔍 Searching for prior art: \"{}\"", idea);
    eprintln!("   keywords: {}", query.keywords.join(", "));

    // ── Phase 1: search sources AND load embedding model concurrently ───
    let t_start = std::time::Instant::now();
    let idea_for_embed = query.idea.clone();
    let (search_result, ranker_result) = tokio::join!(
        patent::sources::search_all(&query),
        tokio::task::spawn_blocking(move || {
            let mut ranker = patent::rank::Ranker::new()?;
            let query_emb = ranker.embed_query(&idea_for_embed)?;
            Ok::<_, patent::Error>((ranker, query_emb))
        })
    );

    let patent::sources::SearchOutcome {
        matches: raw_matches,
        reached,
        failed,
    } = search_result;
    let (mut ranker, query_emb) = ranker_result.expect("embedding task panicked")?;

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
    let limit = args.limit as usize;
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
    // A verdict shown without an AI judgement still carries the integrity
    // caveat and the full sources-checked / not-reached transparency.
    let fallback_verdict = |headline: &str| patent::Verdict {
        level: patent::Saturation::Open,
        headline: headline.to_string(),
        gaps: vec![],
        sources_checked: reached.clone(),
        sources_failed: failed.clone(),
        caveat: patent::verdict::CAVEAT.to_string(),
    };

    let best_sim = ranked.first().map_or(0.0, |m| m.similarity);
    let verdict = if best_sim < MIN_RELEVANCE {
        eprintln!(
            "⚠  Best similarity {:.2} < {:.2} — skipping verdict",
            best_sim, MIN_RELEVANCE,
        );
        fallback_verdict(
            "Nothing relevant turned up in the sources checked. \
             The query may not describe a recognized software tool — \
             try rephrasing with specific technical terms.",
        )
    } else {
        let t_verdict = std::time::Instant::now();
        eprintln!("🧠 Generating verdict via Ollama ({})…", args.model);
        let ollama = patent::ollama::Ollama::new(patent::ollama::DEFAULT_ENDPOINT, &args.model);
        match patent::verdict::assess(&ollama, &query, &ranked, reached.clone(), failed.clone())
            .await
        {
            Ok(v) => {
                eprintln!("   verdict in {:.1}s", t_verdict.elapsed().as_secs_f64());
                v
            }
            Err(patent::Error::OllamaUnreachable(ref addr)) => {
                eprintln!(
                    "⚠  Ollama not reachable at {addr} — showing results without verdict.\n   \
                     Run `ollama serve` and `ollama pull {model}` to enable AI verdicts.",
                    model = args.model,
                );
                fallback_verdict(
                    "Verdict unavailable — Ollama not reachable. \
                     Results are ranked by semantic similarity only.",
                )
            }
            Err(patent::Error::OllamaModel(ref why)) => {
                eprintln!(
                    "⚠  {why} — showing results without an AI verdict.\n   \
                     Run `ollama pull {model}` to enable AI verdicts.",
                    model = args.model,
                );
                fallback_verdict(
                    "Verdict unavailable — the requested Ollama model isn't available. \
                     Results are ranked by semantic similarity only.",
                )
            }
            Err(e) => return Err(e.into()),
        }
    };

    eprintln!("⏱  total: {:.1}s", t_start.elapsed().as_secs_f64());

    // ── Phase 4: output ─────────────────────────────────────────────────
    // The TUI needs a real terminal; when stdout is piped or redirected, fall
    // back to JSON rather than panicking on terminal initialization.
    let want_json = args.json || !std::io::stdout().is_terminal();
    if want_json {
        if !args.json {
            eprintln!("ℹ  stdout is not a terminal — emitting JSON (pass --json to silence this).");
        }
        let output = serde_json::json!({
            "query": idea,
            "verdict": verdict,
            "matches": ranked,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        tui::run(&idea, &verdict, &ranked)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_idea_accepts_a_normal_idea() {
        assert!(validate_idea("a cli tool to kill a process on a given port").is_ok());
    }

    #[test]
    fn validate_idea_rejects_empty_and_vague() {
        assert!(validate_idea("").is_err());
        assert!(validate_idea("   ").is_err());
        assert!(validate_idea("ab cd").is_err());
        // Regression: short ASCII gibberish must NOT slip through the CJK
        // fallback (which only applies to non-ASCII, space-free scripts).
        assert!(validate_idea("foobar").is_err());
        assert!(validate_idea("123456").is_err());
        assert!(validate_idea("hello world").is_err());
    }

    #[test]
    fn validate_idea_accepts_non_ascii_scripts() {
        // Cyrillic (space-delimited) and CJK (no spaces) must both be accepted.
        assert!(validate_idea("инструмент для управления процессами на порту").is_ok());
        assert!(validate_idea("端口杀手命令行工具").is_ok());
    }
}
