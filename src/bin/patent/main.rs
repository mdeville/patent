//! `patent` binary — thin CLI/TUI shell over the `patent` library.

mod cli;
mod tui;

use clap::Parser;
use cli::Cli;

fn build_query(idea: &str) -> patent::Query {
    let keywords: Vec<String> = idea
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .map(|w| w.to_lowercase())
        .collect();
    patent::Query {
        idea: idea.to_string(),
        keywords,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // 1. idea -> Query
    let query = build_query(&args.idea);
    eprintln!("🔍 Searching for prior art: \"{}\"", args.idea);
    eprintln!("   keywords: {}", query.keywords.join(", "));

    // 2. sources::search_all -> Vec<Match>
    let (raw_matches, reached) = patent::sources::search_all(&query).await;
    eprintln!(
        "   {} matches from {} sources: {}",
        raw_matches.len(),
        reached.len(),
        reached
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 3. rank::rank -> top-N
    eprintln!("📊 Ranking (embedding + cosine similarity)…");
    let ranked = patent::rank::rank(&query, raw_matches, args.limit)?;

    // 4. verdict via Ollama
    eprintln!("🧠 Generating verdict via Ollama ({})…", args.model);
    let ollama = patent::ollama::Ollama::new(patent::ollama::DEFAULT_ENDPOINT, &args.model);
    let verdict = patent::verdict::assess(&ollama, &query, &ranked, reached.clone()).await?;

    // 5. output
    if args.json {
        let output = serde_json::json!({
            "verdict": verdict,
            "matches": ranked,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        tui::run(&args.idea, &verdict, &ranked)?;
    }

    Ok(())
}
