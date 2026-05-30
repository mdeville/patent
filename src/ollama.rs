//! Minimal Ollama client (`localhost:11434/api/generate`).
//!
//! Returns a clear, actionable error when the daemon is unreachable.

/// Default Ollama endpoint.
pub const DEFAULT_ENDPOINT: &str = "http://localhost:11434";

/// Default generation model. `qwen2.5` (7B) for quality; `qwen2.5:3b` if RAM is
/// tight. Override via `--model`.
pub const DEFAULT_MODEL: &str = "qwen2.5";

/// A thin handle over the Ollama generate API.
#[derive(Debug, Clone)]
pub struct Ollama {
    endpoint: String,
    model: String,
    client: reqwest::Client,
}

impl Ollama {
    pub fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Send a prompt to `/api/generate` and return the completion text.
    pub async fn generate(&self, _prompt: &str) -> crate::Result<String> {
        let _ = (&self.endpoint, &self.model, &self.client);
        todo!("M4: POST /api/generate; map connection errors to Error::OllamaUnreachable")
    }
}

impl Default for Ollama {
    fn default() -> Self {
        Self::new(DEFAULT_ENDPOINT, DEFAULT_MODEL)
    }
}
