//! Verdict generation (M4).
//!
//! Builds a prompt from the ranked matches and asks Ollama for a scoped verdict.
//! The prompt **forbids claiming non-existence**: results are always phrased as
//! "found in the sources checked", and a clean result means "keep looking before
//! committing", never a green light.

use crate::model::{Match, Query, Source, Verdict};
use crate::ollama::Ollama;

/// The fixed humble caveat shown on every verdict. Never weaken this.
pub const CAVEAT: &str = "Not proof it doesn't exist — only that nothing close turned up \
in the sources checked. Keep looking (web, app stores, niche communities) before committing.";

/// Build the Ollama prompt enforcing the integrity rules.
pub fn build_prompt(_query: &Query, _matches: &[Match]) -> String {
    todo!("M4: construct integrity-scoped prompt")
}

/// Produce a [`Verdict`] from ranked matches via Ollama.
pub async fn assess(
    _ollama: &Ollama,
    _query: &Query,
    _matches: &[Match],
    _sources_checked: Vec<Source>,
) -> crate::Result<Verdict> {
    todo!("M4: build prompt -> generate -> parse -> Verdict (with CAVEAT)")
}
