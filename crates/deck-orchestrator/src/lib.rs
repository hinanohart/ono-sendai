//! deck-orchestrator — the mpsc hub.
//!
//! Owns the `tokio::sync::mpsc` channels that connect the TUI to the LLM,
//! MCP, store and sandbox tasks. The TUI sends [`Command`]s in; each task
//! emits [`Event`]s out; a broadcast channel fans events to anyone who
//! subscribes (TUI, log file, future remote dashboard).

use std::sync::Arc;

use deck_core::{LlmBackend, Message, Role, SessionId, Store, ToolCall, ToolResult};
use futures::StreamExt;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, warn};

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

/// Lightweight handle the TUI (and any other client) keeps. Cloneable;
/// internally just channel handles + a broadcast subscription factory.
#[derive(Clone)]
pub struct Handle {
    commands_tx: mpsc::Sender<Command>,
    events_tx: broadcast::Sender<Event>,
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").finish_non_exhaustive()
    }
}

impl Handle {
    pub async fn submit(&self, cmd: Command) -> Result<(), mpsc::error::SendError<Command>> {
        self.commands_tx.send(cmd).await
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events_tx.subscribe()
    }
}

/// The owned runtime piece: holds the receiver and spawned task.
#[derive(Debug)]
pub struct Runtime {
    pub handle: Handle,
    join: tokio::task::JoinHandle<()>,
}

impl Runtime {
    pub fn spawn(llm: Arc<dyn LlmBackend>, store: Arc<dyn Store>, model: String) -> Self {
        let (commands_tx, commands_rx) = mpsc::channel::<Command>(64);
        let (events_tx, _) = broadcast::channel::<Event>(256);
        let handle = Handle {
            commands_tx,
            events_tx: events_tx.clone(),
        };
        let join = tokio::spawn(run_loop(commands_rx, events_tx, llm, store, model));
        Self { handle, join }
    }

    pub async fn shutdown(self) {
        let _ = self.handle.commands_tx.send(Command::Shutdown).await;
        let _ = self.join.await;
    }
}

async fn run_loop(
    mut commands_rx: mpsc::Receiver<Command>,
    events_tx: broadcast::Sender<Event>,
    llm: Arc<dyn LlmBackend>,
    store: Arc<dyn Store>,
    model: String,
) {
    while let Some(cmd) = commands_rx.recv().await {
        match cmd {
            Command::Shutdown => break,
            Command::UserMessage { session, content } => {
                if let Err(e) = handle_user_message(
                    &events_tx,
                    llm.as_ref(),
                    store.as_ref(),
                    &model,
                    session,
                    content,
                )
                .await
                {
                    let _ = events_tx.send(Event::Error {
                        message: e.to_string(),
                    });
                }
            }
            Command::ApproveTool { call_id } | Command::DenyTool { call_id } => {
                warn!(call_id, "tool approval not wired in Phase 2");
            }
        }
    }
}

async fn handle_user_message(
    events_tx: &broadcast::Sender<Event>,
    llm: &dyn LlmBackend,
    store: &dyn Store,
    model: &str,
    session: SessionId,
    content: String,
) -> deck_core::Result<()> {
    let user_msg = Message {
        role: Role::User,
        content,
        tool_calls: vec![],
    };
    store.append(session, &user_msg).await?;
    let history = store.load(session).await?;
    let mut stream = llm.stream(model, &history).await?;
    let mut accumulated = String::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(delta) => {
                if !delta.content.is_empty() {
                    accumulated.push_str(&delta.content);
                    let _ = events_tx.send(Event::AssistantDelta {
                        session,
                        text: delta.content,
                    });
                }
            }
            Err(e) => {
                error!(error = %e, "llm stream error");
                let _ = events_tx.send(Event::Error {
                    message: e.to_string(),
                });
                return Err(e);
            }
        }
    }
    let asst_msg = Message {
        role: Role::Assistant,
        content: accumulated,
        tool_calls: vec![],
    };
    store.append(session, &asst_msg).await?;
    let _ = events_tx.send(Event::AssistantTurn {
        session,
        message: asst_msg,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use deck_core::{DeckError, Message, Role};
    use futures::stream::{self, BoxStream, StreamExt as _};
    use tokio::sync::Mutex as AsyncMutex;

    struct EchoLlm;
    #[async_trait]
    impl LlmBackend for EchoLlm {
        fn id(&self) -> String {
            "echo".into()
        }
        async fn complete(&self, _model: &str, messages: &[Message]) -> deck_core::Result<Message> {
            let last = messages
                .last()
                .cloned()
                .ok_or_else(|| DeckError::Llm("empty".into()))?;
            Ok(Message {
                role: Role::Assistant,
                content: format!("echo:{}", last.content),
                tool_calls: vec![],
            })
        }
        async fn stream(
            &self,
            _model: &str,
            messages: &[Message],
        ) -> deck_core::Result<BoxStream<'static, deck_core::Result<Message>>> {
            let last = messages
                .last()
                .cloned()
                .ok_or_else(|| DeckError::Llm("empty".into()))?;
            let chunks: Vec<deck_core::Result<Message>> = format!("echo:{}", last.content)
                .chars()
                .map(|c| {
                    Ok(Message {
                        role: Role::Assistant,
                        content: c.to_string(),
                        tool_calls: vec![],
                    })
                })
                .collect();
            Ok(stream::iter(chunks).boxed())
        }
    }

    #[derive(Default, Clone)]
    struct MemStore {
        inner: Arc<AsyncMutex<std::collections::HashMap<SessionId, Vec<Message>>>>,
    }
    #[async_trait]
    impl Store for MemStore {
        async fn append(&self, s: SessionId, m: &Message) -> deck_core::Result<()> {
            self.inner
                .lock()
                .await
                .entry(s)
                .or_default()
                .push(m.clone());
            Ok(())
        }
        async fn load(&self, s: SessionId) -> deck_core::Result<Vec<Message>> {
            Ok(self.inner.lock().await.get(&s).cloned().unwrap_or_default())
        }
        async fn list(&self) -> deck_core::Result<Vec<SessionId>> {
            Ok(self.inner.lock().await.keys().copied().collect())
        }
    }

    #[tokio::test]
    async fn user_message_produces_assistant_turn() {
        let llm: Arc<dyn LlmBackend> = Arc::new(EchoLlm);
        let store: Arc<dyn Store> = Arc::new(MemStore::default());
        let rt = Runtime::spawn(llm, store.clone(), "test-model".into());
        let mut rx = rt.handle.subscribe();
        let session = SessionId::new();
        rt.handle
            .submit(Command::UserMessage {
                session,
                content: "hi".into(),
            })
            .await
            .unwrap();
        let mut got_turn = false;
        for _ in 0..100 {
            match tokio::time::timeout(std::time::Duration::from_millis(500), rx.recv()).await {
                Ok(Ok(Event::AssistantTurn { message, .. })) => {
                    assert!(message.content.contains("echo:hi"));
                    got_turn = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(got_turn);
        rt.shutdown().await;
        let history = store.load(session).await.unwrap();
        assert_eq!(history.len(), 2);
    }
}
