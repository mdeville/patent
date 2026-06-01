use patent::model::{Match, Query, Saturation, Source};
use patent::ollama::Ollama;
use patent::verdict::{self, CAVEAT};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn query() -> Query {
    Query {
        idea: "a cli to kill processes on a port".to_string(),
        keywords: vec!["kill".into(), "port".into()],
    }
}

fn checked() -> Vec<Source> {
    vec![Source::Npm, Source::CratesIo]
}

fn sample_matches() -> Vec<Match> {
    vec![
        Match {
            name: "kill-port".to_string(),
            source: Source::Npm,
            url: "https://npmjs.com/package/kill-port".to_string(),
            description: "Kill process on a port".to_string(),
            popularity: Some(50_000),
            similarity: 0.85,
        },
        Match {
            name: "fkill-cli".to_string(),
            source: Source::Npm,
            url: "https://npmjs.com/package/fkill-cli".to_string(),
            description: "Fabulously kill processes".to_string(),
            popularity: Some(10_000),
            similarity: 0.60,
        },
    ]
}

// -- build_prompt tests -------------------------------------------------------

#[test]
fn prompt_contains_the_idea() {
    let prompt = verdict::build_prompt(&query(), &sample_matches(), &checked());
    assert!(
        prompt.contains("a cli to kill processes on a port"),
        "prompt must include the user's idea"
    );
}

#[test]
fn prompt_contains_match_names() {
    let prompt = verdict::build_prompt(&query(), &sample_matches(), &checked());
    assert!(prompt.contains("kill-port"));
    assert!(prompt.contains("fkill-cli"));
}

#[test]
fn prompt_forbids_asserting_absence() {
    let prompt = verdict::build_prompt(&query(), &sample_matches(), &checked());
    let lower = prompt.to_lowercase();
    assert!(
        lower.contains("never") || lower.contains("do not") || lower.contains("must not"),
        "prompt must forbid claiming non-existence"
    );
}

#[test]
fn prompt_requires_json_output() {
    let prompt = verdict::build_prompt(&query(), &sample_matches(), &checked());
    let lower = prompt.to_lowercase();
    assert!(lower.contains("json"), "prompt must ask for JSON output");
}

#[test]
fn prompt_with_no_matches_still_valid() {
    let prompt = verdict::build_prompt(&query(), &[], &checked());
    assert!(!prompt.is_empty());
    assert!(prompt.contains("a cli to kill processes on a port"));
}

#[test]
fn prompt_names_only_the_sources_actually_checked() {
    // Integrity: the model must only be told about coverage that really
    // happened. With HN not in the reached set, it must not be named.
    let prompt = verdict::build_prompt(&query(), &sample_matches(), &checked());
    assert!(prompt.contains("npm"), "must name a reached source");
    assert!(prompt.contains("crates.io"), "must name a reached source");
    assert!(
        !prompt.contains("Hacker News"),
        "must not name a source that wasn't reached"
    );
}

// -- assess() end-to-end via wiremock -----------------------------------------

fn ollama_response(level: &str, headline: &str, gaps: &[&str]) -> serde_json::Value {
    let model_json = json!({
        "level": level,
        "headline": headline,
        "gaps": gaps,
    });
    json!({
        "response": model_json.to_string(),
        "done": true,
    })
}

async fn mock_ollama(response: serde_json::Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn assess_returns_verdict_with_caveat() {
    let server = mock_ollama(ollama_response(
        "Saturated",
        "Lots of prior art found in the sources checked.",
        &["no Windows support in existing tools"],
    ))
    .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let sources = checked();
    let v = verdict::assess(
        &ollama,
        &query(),
        &sample_matches(),
        sources.clone(),
        vec![],
    )
    .await
    .unwrap();

    assert_eq!(v.level, Saturation::Saturated);
    assert!(v.headline.contains("prior art"));
    assert_eq!(v.gaps.len(), 1);
    assert_eq!(v.sources_checked, sources);
    assert_eq!(v.caveat, CAVEAT);
}

#[tokio::test]
async fn assess_parses_open_level() {
    let server = mock_ollama(ollama_response(
        "Open",
        "Nothing close found in the sources checked.",
        &[],
    ))
    .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &[], vec![Source::GitHub], vec![])
        .await
        .unwrap();

    assert_eq!(v.level, Saturation::Open);
}

#[tokio::test]
async fn assess_parses_crowded_level() {
    let server = mock_ollama(ollama_response(
        "Crowded",
        "A few adjacent tools exist.",
        &["gap one", "gap two"],
    ))
    .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(
        &ollama,
        &query(),
        &sample_matches(),
        vec![Source::Npm],
        vec![],
    )
    .await
    .unwrap();

    assert_eq!(v.level, Saturation::Crowded);
    assert_eq!(v.gaps.len(), 2);
}

#[tokio::test]
async fn assess_handles_json_wrapped_in_markdown_fence() {
    let fenced = "```json\n{\"level\":\"Open\",\"headline\":\"Nothing found.\",\"gaps\":[]}\n```";
    let server = mock_ollama(json!({"response": fenced, "done": true})).await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &[], vec![Source::GitHub], vec![])
        .await
        .unwrap();
    assert_eq!(v.level, Saturation::Open);
}

#[tokio::test]
async fn assess_rejects_garbage_response() {
    let server = mock_ollama(json!({"response": "I don't know what JSON is", "done": true})).await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let err = verdict::assess(&ollama, &query(), &[], vec![], vec![])
        .await
        .unwrap_err();
    assert!(matches!(err, patent::Error::Parse(_)));
}

#[tokio::test]
async fn assess_floors_level_when_model_underrates_a_crowded_space() {
    // The model says "Open" but the similarity data shows two close matches
    // (0.85, 0.60). The level must be floored up to Crowded, and because the
    // model misjudged, its headline is replaced with a safe data-derived one.
    let server = mock_ollama(ollama_response(
        "Open",
        "This is a brand-new idea about a testing framework.",
        &[],
    ))
    .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &sample_matches(), checked(), vec![])
        .await
        .unwrap();

    assert_eq!(
        v.level,
        Saturation::Crowded,
        "two >=0.55 matches => Crowded"
    );
    assert!(
        !v.headline.contains("testing framework"),
        "a floored level must not keep the model's misjudged headline"
    );
    assert!(v.headline.to_lowercase().contains("sources checked"));
}

#[tokio::test]
async fn assess_replaces_absence_claiming_headline() {
    // No matches => level stays Open, so the model's headline is kept *unless*
    // it asserts non-existence — which it does here, and must be replaced.
    let server = mock_ollama(ollama_response(
        "Open",
        "This tool does not exist and there is no prior art for it.",
        &[],
    ))
    .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &[], vec![Source::GitHub], vec![])
        .await
        .unwrap();

    let lower = v.headline.to_lowercase();
    assert!(
        !lower.contains("does not exist"),
        "absence claim must be scrubbed"
    );
    assert!(
        !lower.contains("no prior art"),
        "absence claim must be scrubbed"
    );
    assert_eq!(v.caveat, CAVEAT);
}

#[tokio::test]
async fn assess_drops_gaps_that_assert_absence() {
    // A gap bullet that smuggles an absence claim must be filtered out; a
    // legitimate gap is kept.
    let server = mock_ollama(ollama_response(
        "Open",
        "A few adjacent tools turned up in the sources checked.",
        &[
            "no existing tool supports Windows, and there is no prior art for this",
            "none of the matches offer an async API",
        ],
    ))
    .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &[], vec![Source::GitHub], vec![])
        .await
        .unwrap();

    assert_eq!(v.gaps.len(), 1, "the absence-asserting gap must be dropped");
    assert!(v.gaps[0].contains("async API"));
    for g in &v.gaps {
        assert!(!g.to_lowercase().contains("no prior art"));
        assert!(!g.to_lowercase().contains("no existing tool"));
    }
}

#[tokio::test]
async fn assess_scrubs_broadened_absence_phrasings() {
    // Phrasings beyond the obvious ones must also be caught.
    for headline in [
        "This has not been implemented anywhere yet.",
        "This is unprecedented — no similar tool exists.",
        "There is no existing software like this.",
    ] {
        let server = mock_ollama(ollama_response("Open", headline, &[])).await;
        let ollama = Ollama::new(server.uri(), "qwen2.5");
        let v = verdict::assess(&ollama, &query(), &[], vec![Source::GitHub], vec![])
            .await
            .unwrap();
        let lower = v.headline.to_lowercase();
        assert!(
            !lower.contains("has not been implemented")
                && !lower.contains("unprecedented")
                && !lower.contains("no similar tool")
                && !lower.contains("no existing software"),
            "absence headline survived: {:?}",
            v.headline
        );
    }
}

#[tokio::test]
async fn assess_threads_failed_sources_into_verdict() {
    let server = mock_ollama(ollama_response("Open", "Nothing close turned up.", &[])).await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(
        &ollama,
        &query(),
        &[],
        vec![Source::GitHub],
        vec![Source::PyPI, Source::CratesIo],
    )
    .await
    .unwrap();

    assert_eq!(v.sources_failed, vec![Source::PyPI, Source::CratesIo]);
}

// -- from_data() — the --fast / no-LLM path -----------------------------------

#[test]
fn from_data_floors_level_against_similarity() {
    // Two matches at 0.85 and 0.60 => at least two >= 0.55 => Crowded, derived
    // from the similarity data alone with no model in the loop. The no-LLM path
    // must never under-rate a populated space into a green "Open".
    let v = verdict::from_data(&sample_matches(), checked(), vec![]);
    assert_eq!(v.level, Saturation::Crowded);
    assert!(v.gaps.is_empty(), "no model => no gaps");
    assert_eq!(v.caveat, CAVEAT);
    assert_eq!(v.sources_checked, checked());
}

#[test]
fn from_data_open_when_nothing_close() {
    let v = verdict::from_data(&[], vec![Source::GitHub], vec![]);
    assert_eq!(v.level, Saturation::Open);
    assert_eq!(v.caveat, CAVEAT);
}

#[test]
fn from_data_never_asserts_absence() {
    // The integrity rule holds on the no-LLM path too: even with zero matches
    // the headline must never claim the idea doesn't exist anywhere.
    let v = verdict::from_data(&[], vec![Source::GitHub], vec![]);
    let lower = v.headline.to_lowercase();
    for phrase in [
        "does not exist",
        "no prior art",
        "never been",
        "unprecedented",
    ] {
        assert!(
            !lower.contains(phrase),
            "absence claim in fast headline: {:?}",
            v.headline
        );
    }
}

#[test]
fn from_data_threads_failed_sources() {
    let v = verdict::from_data(&sample_matches(), checked(), vec![Source::PyPI]);
    assert_eq!(v.sources_failed, vec![Source::PyPI]);
}
