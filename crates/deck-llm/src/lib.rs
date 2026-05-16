//! deck-llm — LLM backend implementations.
//!
//! Phase 1 ships an [`OllamaBackend`] talking to a local Ollama daemon over
//! HTTP. A `llama-cpp` in-process backend lives behind a `llama-cpp` feature
//! flag and is wired in Phase 2.

pub mod mock;
pub mod ollama;

pub use mock::MockBackend;
pub use ollama::OllamaBackend;

use deck_core::LlmBackend;

/// Build a backend from a `[llm]` config block.
///
/// Errors if the backend identifier is unknown.
pub fn from_config(cfg: &deck_core::config::LlmConfig) -> deck_core::Result<Box<dyn LlmBackend>> {
    match cfg.backend.as_str() {
        "ollama" => {
            let b = OllamaBackend::new(
                cfg.endpoint.clone(),
                std::time::Duration::from_secs(cfg.timeout_secs),
            )?;
            Ok(Box::new(b))
        }
        "mock" => Ok(Box::new(MockBackend::default())),
        other => Err(deck_core::DeckError::Llm(format!(
            "unknown backend: {other}"
        ))),
    }
}
