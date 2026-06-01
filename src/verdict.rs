//! Verdict generation (M4).
//!
//! Builds a prompt from the ranked matches and asks Ollama for a scoped verdict.
//! The prompt **forbids claiming non-existence**: results are always phrased as
//! "found in the sources checked", and a clean result means "keep looking before
//! committing", never a green light.

use crate::model::{Match, Query, Saturation, Source, Verdict};
use crate::ollama::Ollama;

/// The fixed humble caveat shown on every verdict. Never weaken this.
pub const CAVEAT: &str = "Not proof it doesn't exist — only that nothing close turned up \
in the sources checked. Keep looking (web, app stores, niche communities) before committing.";

/// Build the Ollama prompt enforcing the integrity rules.
pub fn build_prompt(query: &Query, matches: &[Match]) -> String {
    let mut prompt = String::new();

    prompt.push_str(
        "You are a prior-art analyst. The user has an idea for a dev tool and we searched \
         several open-source registries for existing implementations.\n\n",
    );

    prompt.push_str(&format!("## Idea\n{}\n\n", query.idea));

    if matches.is_empty() {
        prompt.push_str("## Matches\nNo matches were found in the sources checked.\n\n");
    } else {
        prompt.push_str("## Matches found (ranked by similarity)\n");
        for m in matches {
            prompt.push_str(&format!(
                "- **{}** ({}, similarity {:.2}): {}\n",
                m.name, m.source, m.similarity, m.description,
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str(
        "## Rules — you MUST follow these\n\
         - You can prove something EXISTS; you must NEVER claim something does not exist.\n\
         - All conclusions must be scoped to \"found in the sources checked\".\n\
         - Do not say \"this doesn't exist\" or \"there is no prior art\" — only that \
           nothing close turned up in the sources checked.\n\n",
    );

    prompt.push_str(
        "## Output\n\
         Respond with ONLY a JSON object (no markdown fences, no commentary):\n\
         ```\n\
         {\n  \
           \"level\": \"Open\" | \"Crowded\" | \"Saturated\",\n  \
           \"headline\": \"one-sentence summary scoped to sources checked\",\n  \
           \"gaps\": [\"gap the user could fill\", ...]\n\
         }\n\
         ```\n\
         - Open: nothing close found in the sources checked.\n\
         - Crowded: a few adjacent things exist.\n\
         - Saturated: the space is densely populated.\n",
    );

    prompt
}

/// Extract JSON from a model response that may be wrapped in markdown fences.
fn extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        let content = after_fence
            .strip_prefix("json")
            .unwrap_or(after_fence)
            .trim_start();
        if let Some(end) = content.find("```") {
            return content[..end].trim();
        }
    }
    trimmed
}

/// Parse the model's JSON response into the verdict fields we need.
fn parse_verdict(raw: &str, sources_checked: Vec<Source>) -> crate::Result<Verdict> {
    let json_str = extract_json(raw);

    let v: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| crate::Error::Parse(e.to_string()))?;

    let level = match v["level"].as_str() {
        Some("Open") => Saturation::Open,
        Some("Crowded") => Saturation::Crowded,
        Some("Saturated") => Saturation::Saturated,
        other => return Err(crate::Error::Parse(format!("invalid level: {:?}", other))),
    };

    let headline = v["headline"]
        .as_str()
        .ok_or_else(|| crate::Error::Parse("missing 'headline'".into()))?
        .to_string();

    let gaps = v["gaps"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|g| g.as_str().map(String::from))
        .collect();

    Ok(Verdict {
        level,
        headline,
        gaps,
        sources_checked,
        caveat: CAVEAT.to_string(),
    })
}

/// Produce a [`Verdict`] from ranked matches via Ollama.
pub async fn assess(
    ollama: &Ollama,
    query: &Query,
    matches: &[Match],
    sources_checked: Vec<Source>,
) -> crate::Result<Verdict> {
    let prompt = build_prompt(query, matches);
    let raw = ollama.generate(&prompt).await?;
    parse_verdict(&raw, sources_checked)
}
