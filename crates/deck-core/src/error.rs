//! Crate-wide error type. Libraries use `thiserror` for typed errors; the
//! binary crate uses `anyhow` for application-level chaining.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeckError {
    #[error("config: {0}")]
    Config(String),

    #[error("llm backend: {0}")]
    Llm(String),

    #[error("mcp: {0}")]
    Mcp(String),

    #[error("store: {0}")]
    Store(String),

    #[error("sandbox: {0}")]
    Sandbox(String),

    #[error("orchestrator: {0}")]
    Orchestrator(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T, E = DeckError> = std::result::Result<T, E>;
