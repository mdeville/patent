//! Text-generation backend used to write the verdict.
//!
//! Two implementations: [`crate::ollama::Ollama`] (local Ollama) and
//! [`crate::openai::OpenAi`] (any OpenAI-compatible chat API).

/// A backend that turns a prompt into completion text.
#[async_trait::async_trait]
pub trait Llm: Send + Sync {
    /// Generate a completion for `prompt`.
    async fn generate(&self, prompt: &str) -> crate::Result<String>;

    /// Short name shown in progress output, e.g. "Ollama".
    fn label(&self) -> &str;
}
