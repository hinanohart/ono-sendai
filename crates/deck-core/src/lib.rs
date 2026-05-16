//! deck-core — shared domain types, traits and configuration.
//!
//! Every other `deck-*` crate depends on this one. We deliberately keep it
//! free of heavy runtime deps (no tokio, no rusqlite, no ratatui) so that
//! `cargo check -p deck-core` stays under a second.

pub mod config;
pub mod error;
pub mod message;
pub mod traits;

pub use config::Config;
pub use error::{DeckError, Result};
pub use message::{Message, Role, SessionId, ToolCall, ToolResult};
pub use traits::{LlmBackend, McpClient, Sandbox, Store};
