//! Verdict generation.
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
        "You are a prior-art analyst for SOFTWARE DEVELOPER TOOLS ONLY. The user has an \
         idea for a dev tool and we searched open-source registries (crates.io, npm, PyPI, \
         GitHub, Hacker News) for existing implementations.\n\n",
    );

    prompt.push_str(&format!("## Idea\n{}\n\n", query.idea));

    if matches.is_empty() {
        prompt.push_str("## Matches\nNo matches were found in the sources checked.\n\n");
    } else {
        let top10: Vec<&Match> = matches.iter().take(10).collect();
        let avg_sim: f32 = top10.iter().map(|m| m.similarity).sum::<f32>() / top10.len() as f32;

        prompt.push_str("## Matches found (ranked by cosine similarity to the idea)\n");
        prompt.push_str(&format!(
            "Top-10 average similarity: {:.2} (scale: 0.0 = unrelated, 0.5 = tangential, \
             0.7+ = strong match)\n\n",
            avg_sim,
        ));
        for m in matches.iter().take(15) {
            prompt.push_str(&format!(
                "- **{}** ({}, sim {:.2}): {}\n",
                m.name, m.source, m.similarity, m.description,
            ));
        }
        if matches.len() > 15 {
            prompt.push_str(&format!(
                "- … and {} more with lower similarity\n",
                matches.len() - 15
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str(
        "## Rules — you MUST follow these\n\
         - You can prove something EXISTS; you must NEVER claim something does not exist.\n\
         - All conclusions must be scoped to \"found in the sources checked\".\n\
         - Do not say \"this doesn't exist\" or \"there is no prior art\" — only that \
           nothing close turned up in the sources checked.\n\
         - If the idea is NOT about software, developer tools, or programming, respond \
           with level \"Open\" and headline \"This does not appear to be a software tool \
           idea — patent searches developer tool registries only.\"\n\
         - Focus ONLY on matches that directly address the SPECIFIC feature described in \
           the idea. Generic or tangential tools (e.g. a generic linter when the idea is \
           a specific kind of linter) do NOT count as prior art.\n\n",
    );

    prompt.push_str(
        "## How to choose the level\n\
         Use the similarity scores — they measure how closely each match relates to the idea:\n\
         - **Open**: no match has similarity >= 0.55, OR matches are only tangentially \
           related (they share a category but not the specific feature).\n\
         - **Crowded**: at least 2-3 matches with similarity >= 0.55 that directly \
           address the same problem.\n\
         - **Saturated**: 5+ strong matches (>= 0.60) covering the idea with little room \
           for differentiation.\n\n",
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
         ```\n",
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

    let gaps = match v["gaps"].as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|g| g.as_str().map(String::from))
            .collect(),
        None => vec![],
    };

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
