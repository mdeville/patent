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
    let prompt = verdict::build_prompt(&query(), &sample_matches());
    assert!(
        prompt.contains("a cli to kill processes on a port"),
        "prompt must include the user's idea"
    );
}

#[test]
fn prompt_contains_match_names() {
    let prompt = verdict::build_prompt(&query(), &sample_matches());
    assert!(prompt.contains("kill-port"));
    assert!(prompt.contains("fkill-cli"));
}

#[test]
fn prompt_forbids_asserting_absence() {
    let prompt = verdict::build_prompt(&query(), &sample_matches());
    let lower = prompt.to_lowercase();
    assert!(
        lower.contains("never") || lower.contains("do not") || lower.contains("must not"),
        "prompt must forbid claiming non-existence"
    );
}

#[test]
fn prompt_requires_json_output() {
    let prompt = verdict::build_prompt(&query(), &sample_matches());
    let lower = prompt.to_lowercase();
    assert!(lower.contains("json"), "prompt must ask for JSON output");
}

#[test]
fn prompt_with_no_matches_still_valid() {
    let prompt = verdict::build_prompt(&query(), &[]);
    assert!(!prompt.is_empty());
    assert!(prompt.contains("a cli to kill processes on a port"));
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

#[tokio::test]
async fn assess_returns_verdict_with_caveat() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_response(
            "Saturated",
            "Lots of prior art found in the sources checked.",
            &["no Windows support in existing tools"],
        )))
        .mount(&server)
        .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let sources = vec![Source::Npm, Source::CratesIo];
    let v = verdict::assess(&ollama, &query(), &sample_matches(), sources.clone())
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
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_response(
            "Open",
            "Nothing close found in the sources checked.",
            &[],
        )))
        .mount(&server)
        .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &[], vec![Source::GitHub])
        .await
        .unwrap();

    assert_eq!(v.level, Saturation::Open);
}

#[tokio::test]
async fn assess_parses_crowded_level() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_response(
            "Crowded",
            "A few adjacent tools exist.",
            &["gap one", "gap two"],
        )))
        .mount(&server)
        .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &sample_matches(), vec![Source::Npm])
        .await
        .unwrap();

    assert_eq!(v.level, Saturation::Crowded);
    assert_eq!(v.gaps.len(), 2);
}

#[tokio::test]
async fn assess_handles_json_wrapped_in_markdown_fence() {
    let fenced = "```json\n{\"level\":\"Open\",\"headline\":\"Nothing found.\",\"gaps\":[]}\n```";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"response": fenced, "done": true})),
        )
        .mount(&server)
        .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let v = verdict::assess(&ollama, &query(), &[], vec![Source::GitHub])
        .await
        .unwrap();
    assert_eq!(v.level, Saturation::Open);
}

#[tokio::test]
async fn assess_rejects_garbage_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"response": "I don't know what JSON is", "done": true})),
        )
        .mount(&server)
        .await;

    let ollama = Ollama::new(server.uri(), "qwen2.5");
    let err = verdict::assess(&ollama, &query(), &[], vec![])
        .await
        .unwrap_err();
    assert!(matches!(err, patent::Error::Parse(_)));
}
