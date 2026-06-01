use patent::ollama::Ollama;
use serde_json::json;
use wiremock::matchers::{body_json_string, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ollama_for(server: &MockServer, model: &str) -> Ollama {
    Ollama::new(server.uri(), model)
}

fn generate_response(text: &str) -> serde_json::Value {
    json!({ "response": text, "done": true })
}

#[tokio::test]
async fn generate_returns_response_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(generate_response("hello world")))
        .mount(&server)
        .await;

    let result = ollama_for(&server, "qwen2.5")
        .generate("say hi")
        .await
        .unwrap();
    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn generate_sends_model_and_prompt() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .and(body_json_string(
            json!({"model": "qwen2.5", "prompt": "say hi", "stream": false}).to_string(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(generate_response("ok")))
        .expect(1)
        .mount(&server)
        .await;

    ollama_for(&server, "qwen2.5")
        .generate("say hi")
        .await
        .unwrap();
}

#[tokio::test]
async fn generate_maps_connection_error_to_ollama_unreachable() {
    let ollama = Ollama::new("http://127.0.0.1:1", "qwen2.5");
    let err = ollama.generate("hi").await.unwrap_err();
    assert!(
        matches!(err, patent::Error::OllamaUnreachable(_)),
        "expected OllamaUnreachable, got: {err:?}"
    );
}

#[tokio::test]
async fn generate_maps_server_error_to_parse() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/generate"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let err = ollama_for(&server, "qwen2.5")
        .generate("hi")
        .await
        .unwrap_err();
    assert!(
        matches!(err, patent::Error::Parse(_)),
        "expected Parse, got: {err:?}"
    );
}
