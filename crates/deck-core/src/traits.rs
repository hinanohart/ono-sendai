//! Backend-agnostic trait surface. Each `deck-*` crate is a concrete
//! implementation of one or more of these traits; the orchestrator depends
//! only on these abstractions.

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::error::Result;
use crate::message::{Message, ToolCall, ToolResult};

/// A streamable LLM backend (ollama HTTP, llama.cpp in-process, etc).
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Human-readable backend identifier (e.g. `"ollama:llama3.1"`).
    fn id(&self) -> String;

    /// Single-shot completion. Implementations MAY buffer a streamed
    /// response internally — see [`stream`](Self::stream) for the streamed
    /// API.
    async fn complete(&self, model: &str, messages: &[Message]) -> Result<Message>;

    /// Streamed completion. Yields incremental [`Message`] deltas; the last
    /// delta has the final assistant turn.
    async fn stream(
        &self,
        model: &str,
        messages: &[Message],
    ) -> Result<BoxStream<'static, Result<Message>>>;
}

/// An MCP client connected to a single server (one trait instance per
/// declared server). Implementations wrap `rmcp` transports.
#[async_trait]
pub trait McpClient: Send + Sync {
    fn server_name(&self) -> &str;

    /// List the tools the server advertises.
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>>;

    /// Invoke a tool. The orchestrator is responsible for the human-in-the-
    /// loop approval; this trait is the raw RPC.
    async fn call(&self, call: &ToolCall) -> Result<ToolResult>;
}

#[derive(Debug, Clone)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub json_schema: serde_json::Value,
}

/// Encrypted, durable session/context store.
#[async_trait]
pub trait Store: Send + Sync {
    /// Persist (append) a message to a session log.
    async fn append(&self, session: crate::message::SessionId, msg: &Message) -> Result<()>;

    /// Load all messages for a session, in append order.
    async fn load(&self, session: crate::message::SessionId) -> Result<Vec<Message>>;

    /// Enumerate sessions stored in this deck.
    async fn list(&self) -> Result<Vec<crate::message::SessionId>>;
}

/// Sandbox boundary: wrap an MCP server child process so that any tool
/// invocation can only access an explicitly granted set of paths and
/// syscalls.
pub trait Sandbox: Send + Sync {
    /// Returns a tag like `"seccomp+landlock"`, `"unsupported(macos)"`.
    fn availability(&self) -> &'static str;

    /// Whether the current platform can actually enforce a policy.
    fn enforces(&self) -> bool;
}
