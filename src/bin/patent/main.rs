//! `patent` binary — thin CLI/TUI shell over the `patent` library.

mod cli;
mod tui;

use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    // Pipeline (wired up across M1–M5):
    //   1. idea -> Query (keywords)
    //   2. sources::search_all -> Vec<Match>   (concurrent fan-out, M2)
    //   3. rank::rank -> top-N                  (M3)
    //   4. verdict::assess via Ollama           (M4)
    //   5. --json print | tui::run              (M5)
    let _ = (&args.idea, args.limit, &args.model, args.json);
    let _render: fn(&patent::Verdict, &[patent::Match]) -> anyhow::Result<()> = tui::run;

    todo!("wire pipeline across M1-M5")
}
