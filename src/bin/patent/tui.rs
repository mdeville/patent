//! ratatui interface (M5).
//!
//! Header (idea + sources-checked transparency line), verdict panel
//! (🟢/🟡/🔴 + headline + gaps + caveat), and a scrollable/filterable matches
//! table. `↑/↓` scroll, `/` filter, `Enter` open URL, `q` quit.

use patent::model::{Match, Verdict};

/// Render the results interactively. M5 fills this in.
#[allow(dead_code)]
pub fn run(_verdict: &Verdict, _matches: &[Match]) -> anyhow::Result<()> {
    todo!("M5: ratatui + crossterm event loop")
}
