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

/// Render the list of sources actually searched, for the prompt.
fn source_list(sources_checked: &[Source]) -> String {
    if sources_checked.is_empty() {
        return "the selected open-source registries".to_string();
    }
    sources_checked
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build the Ollama prompt enforcing the integrity rules.
///
/// `sources_checked` must be the sources that actually responded — the prompt
/// only ever tells the model about coverage that really happened, so the model
/// can't be steered into claiming a source was searched when it wasn't.
pub fn build_prompt(query: &Query, matches: &[Match], sources_checked: &[Source]) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "You are a prior-art analyst for SOFTWARE DEVELOPER TOOLS ONLY. The user has an \
         idea for a dev tool and we searched these open-source sources for existing \
         implementations: {}.\n\n",
        source_list(sources_checked),
    ));

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
         ```\n\
         The headline MUST describe the user's idea and its closest matches above \
         — never an unrelated tool from the list — and must be scoped to the \
         sources checked. Never claim the idea does not exist or has no prior art.\n",
    );

    prompt
}

/// Phrases that assert non-existence. The integrity rule forbids ever telling a
/// user their idea doesn't exist (we only searched some sources), so if the
/// model emits one of these despite the prompt, we replace the text.
///
/// This is a deliberately broad, conservative backstop: a false positive only
/// downgrades the copy to a safe scoped headline, whereas a false negative is
/// an integrity violation — so we err toward catching more.
const ABSENCE_PHRASES: &[&str] = &[
    "does not exist",
    "doesn't exist",
    "do not exist",
    "don't exist",
    "no prior art",
    "nothing exists",
    "nothing like this",
    "never been built",
    "never been made",
    "never been implemented",
    "has not been built",
    "hasn't been built",
    "has not been implemented",
    "hasn't been implemented",
    "not been implemented",
    "no one has built",
    "no one has made",
    "no one else",
    "nobody else",
    "no one is doing",
    "no such tool",
    "no existing tool",
    "no existing solution",
    "no existing implementation",
    "no similar tool",
    "no similar project",
    "no comparable",
    "no competitors",
    "no alternatives",
    "no equivalent",
    "there is no tool",
    "there are no tools",
    "there is no existing",
    "there is no software",
    "there is no prior",
    "completely novel",
    "entirely new",
    "brand new concept",
    "first of its kind",
    "unprecedented",
];

/// True if `text` asserts that something does not exist.
fn contains_absence_phrase(text: &str) -> bool {
    let lower = text.to_lowercase();
    ABSENCE_PHRASES.iter().any(|p| lower.contains(p))
}

/// A safe, scoped headline derived purely from the data — never asserts absence.
fn data_headline(level: Saturation, matches: &[Match]) -> String {
    let close = matches.iter().filter(|m| m.similarity >= 0.55).count();
    match level {
        Saturation::Saturated => {
            format!("Saturated — {close} closely-related tools turned up in the sources checked.")
        }
        Saturation::Crowded => format!(
            "Crowded — {close} closely-related tool{} turned up in the sources checked.",
            if close == 1 { "" } else { "s" }
        ),
        Saturation::Open => {
            "Nothing close turned up in the sources checked — keep looking before committing."
                .to_string()
        }
    }
}

/// Replace any headline that asserts non-existence with a safe scoped one. This
/// is the code-level guarantee behind the integrity rule; the prompt asks the
/// model to comply, but we never *rely* on it.
fn guard_headline(headline: String, level: Saturation, matches: &[Match]) -> String {
    if contains_absence_phrase(&headline) {
        data_headline(level, matches)
    } else {
        headline
    }
}

/// Floor the model's level against the similarity data so it can never hand out
/// a green-light "Open" when the embeddings clearly show close prior art.
fn floor_level(model_level: Saturation, matches: &[Match]) -> Saturation {
    let strong = matches.iter().filter(|m| m.similarity >= 0.60).count();
    let close = matches.iter().filter(|m| m.similarity >= 0.55).count();
    let data_level = if strong >= 5 {
        Saturation::Saturated
    } else if close >= 2 {
        Saturation::Crowded
    } else {
        Saturation::Open
    };
    model_level.max(data_level)
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

/// Parse the model's JSON response into the verdict fields we need, then apply
/// the two integrity guards: floor the level against the similarity data, and
/// replace any headline that asserts non-existence.
fn parse_verdict(
    raw: &str,
    matches: &[Match],
    sources_checked: Vec<Source>,
    sources_failed: Vec<Source>,
) -> crate::Result<Verdict> {
    let json_str = extract_json(raw);

    let v: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| crate::Error::Parse(e.to_string()))?;

    let model_level = match v["level"].as_str() {
        Some("Open") => Saturation::Open,
        Some("Crowded") => Saturation::Crowded,
        Some("Saturated") => Saturation::Saturated,
        other => return Err(crate::Error::Parse(format!("invalid level: {:?}", other))),
    };

    let raw_headline = v["headline"]
        .as_str()
        .ok_or_else(|| crate::Error::Parse("missing 'headline'".into()))?
        .to_string();

    // Gaps render verbatim, so they get the same absence-claim guard as the
    // headline: a gap that asserts non-existence is dropped rather than shown.
    let gaps: Vec<String> = match v["gaps"].as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|g| g.as_str().map(String::from))
            .filter(|g| !contains_absence_phrase(g))
            .collect(),
        None => vec![],
    };

    // Floor the level against the data. If that raises it, the model misjudged
    // the space, so we don't trust its headline either — derive a safe one.
    let level = floor_level(model_level, matches);
    let headline = if level != model_level {
        data_headline(level, matches)
    } else {
        raw_headline
    };
    let headline = guard_headline(headline, level, matches);

    Ok(Verdict {
        level,
        headline,
        gaps,
        sources_checked,
        sources_failed,
        caveat: CAVEAT.to_string(),
    })
}

/// Build a verdict from the similarity data alone, without calling a model.
///
/// This is the `--fast` path (and any caller that deliberately skips Ollama).
/// The saturation level is derived purely by `floor_level`-ing against the
/// embeddings and the headline is the same safe, scoped, data-only sentence the
/// flooring guard produces — so a no-LLM run still gives an honest signal,
/// never flashes a misleading green "Open" over a clearly-populated space, and
/// still carries the fixed integrity [`CAVEAT`]. Gaps require a model, so they
/// are empty here.
pub fn from_data(
    matches: &[Match],
    sources_checked: Vec<Source>,
    sources_failed: Vec<Source>,
) -> Verdict {
    let level = floor_level(Saturation::Open, matches);
    Verdict {
        headline: data_headline(level, matches),
        level,
        gaps: vec![],
        sources_checked,
        sources_failed,
        caveat: CAVEAT.to_string(),
    }
}

/// Produce a [`Verdict`] from ranked matches via Ollama.
pub async fn assess(
    ollama: &Ollama,
    query: &Query,
    matches: &[Match],
    sources_checked: Vec<Source>,
    sources_failed: Vec<Source>,
) -> crate::Result<Verdict> {
    let prompt = build_prompt(query, matches, &sources_checked);
    let raw = ollama.generate(&prompt).await?;
    parse_verdict(&raw, matches, sources_checked, sources_failed)
}
