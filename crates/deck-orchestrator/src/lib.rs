//! deck-orchestrator — the mpsc hub.
//!
//! Owns the `tokio::sync::mpsc` channels that connect the TUI to the LLM,
//! MCP, store and sandbox tasks. The TUI sends [`Command`]s in; each task
//! emits [`Event`]s out; a broadcast channel fans events to anyone who
//! subscribes (TUI, log file, future remote dashboard).

use deck_core::{Message, SessionId, ToolCall, ToolResult};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub enum Command {
    /// Append a user turn and ask the LLM to continue.
    UserMessage { session: SessionId, content: String },
    /// Approve a pending tool call.
    ApproveTool { call_id: String },
    /// Deny a pending tool call.
    DenyTool { call_id: String },
    /// Graceful shutdown.
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum Event {
    AssistantDelta {
        session: SessionId,
        text: String,
    },
    AssistantTurn {
        session: SessionId,
        message: Message,
    },
    ToolCallProposed {
        call: ToolCall,
    },
    ToolCallResult {
        result: ToolResult,
    },
    Error {
        message: String,
    },
}

/// Construct a fresh orchestrator with bounded channels appropriate for an
/// interactive TUI session.
#[derive(Debug)]
pub struct Orchestrator {
    pub commands_tx: mpsc::Sender<Command>,
    pub commands_rx: mpsc::Receiver<Command>,
    pub events_tx: broadcast::Sender<Event>,
}

impl Orchestrator {
    #[must_use]
    pub fn new() -> Self {
        let (commands_tx, commands_rx) = mpsc::channel(64);
        let (events_tx, _) = broadcast::channel(256);
        Self {
            commands_tx,
            commands_rx,
            events_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events_tx.subscribe()
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn command_roundtrip() {
        let mut orch = Orchestrator::new();
        orch.commands_tx
            .send(Command::Shutdown)
            .await
            .expect("send");
        let recv = orch.commands_rx.recv().await;
        assert!(matches!(recv, Some(Command::Shutdown)));
    }
}
