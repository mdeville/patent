//! Command-line argument parsing.

use clap::Parser;

/// A prior-art search for your code ideas.
#[derive(Debug, Parser)]
#[command(name = "patent", version, about)]
pub struct Cli {
    /// The dev-tool idea to search for, e.g.
    /// "interactive cli to kill whatever's on a port".
    #[arg(required_unless_present = "completions")]
    pub idea: Option<String>,

    /// Max number of matches to keep after ranking.
    #[arg(long, default_value_t = patent::rank::DEFAULT_LIMIT)]
    pub limit: usize,

    /// Ollama model to use for the verdict.
    #[arg(long, default_value = patent::ollama::DEFAULT_MODEL)]
    pub model: String,

    /// Print structured JSON instead of launching the TUI.
    #[arg(long)]
    pub json: bool,

    /// Generate shell completions and exit.
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<clap_complete::Shell>,
}
