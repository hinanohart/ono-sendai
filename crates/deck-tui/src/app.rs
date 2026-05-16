//! Application state + event loop.

use std::io;
use std::time::Duration;

use anyhow::Result;
use deck_orchestrator::{Command as OrchCommand, Event as OrchEvent};
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::event::{Event, EventStream};
use crate::ui;
use crate::AppHandle;

#[derive(Debug)]
pub struct App {
    pub input: String,
    pub log: Vec<String>,
    pub pending_assistant: String,
    pub should_quit: bool,
    pub status: String,
    pub handle: Option<AppHandle>,
}

impl App {
    #[must_use]
    pub fn new(handle: Option<AppHandle>) -> Self {
        let banner = if handle.is_some() {
            "// ono-sendai online. LLM connected. type `:q` to exit.".to_owned()
        } else {
            "// ono-sendai offline-only mode (no LLM). `:q` to exit.".to_owned()
        };
        Self {
            input: String::new(),
            log: vec![banner],
            pending_assistant: String::new(),
            should_quit: false,
            status: format!("v{}", env!("CARGO_PKG_VERSION")),
            handle,
        }
    }

    pub fn handle_input_key(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn handle_backspace(&mut self) {
        self.input.pop();
    }

    /// Process the current input buffer. Returns the user line that should
    /// be forwarded to the orchestrator (if any).
    pub fn handle_enter(&mut self) -> Option<String> {
        if self.input == ":q" {
            self.should_quit = true;
            return None;
        }
        if self.input.is_empty() {
            return None;
        }
        let line = std::mem::take(&mut self.input);
        self.log.push(format!("> {line}"));
        Some(line)
    }

    fn ingest_event(&mut self, ev: OrchEvent) {
        match ev {
            OrchEvent::AssistantDelta { text, .. } => {
                self.pending_assistant.push_str(&text);
            }
            OrchEvent::AssistantTurn { message, .. } => {
                if !self.pending_assistant.is_empty() {
                    self.log.push(format!("< {}", self.pending_assistant));
                    self.pending_assistant.clear();
                } else {
                    self.log.push(format!("< {}", message.content));
                }
            }
            OrchEvent::ToolCallProposed { call } => {
                self.log
                    .push(format!("[tool proposal] {}::{}", call.server, call.tool));
            }
            OrchEvent::ToolCallResult { result } => {
                self.log.push(format!("[tool result] {}", result.call_id));
            }
            OrchEvent::Error { message } => {
                self.log.push(format!("[error] {message}"));
            }
        }
    }

    pub async fn run<B: Backend + io::Write>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let mut events = EventStream::new(Duration::from_millis(16));
        let mut orch_rx = self.handle.as_ref().map(|h| h.handle.subscribe());
        while !self.should_quit {
            terminal.draw(|f| ui::draw(f, self))?;
            tokio::select! {
                ev = events.next() => {
                    match ev {
                        Some(Event::Key(c)) => self.handle_input_key(c),
                        Some(Event::Enter) => {
                            if let Some(line) = self.handle_enter() {
                                if let Some(h) = &self.handle {
                                    let _ = h.handle.submit(OrchCommand::UserMessage {
                                        session: h.session,
                                        content: line,
                                    }).await;
                                } else {
                                    self.log.push("  (offline mode: not forwarded)".into());
                                }
                            }
                        }
                        Some(Event::Backspace) => self.handle_backspace(),
                        Some(Event::Quit) => self.should_quit = true,
                        Some(Event::Tick) | None => {}
                    }
                }
                Some(Ok(ev)) = async {
                    match orch_rx.as_mut() {
                        Some(r) => Some(r.recv().await),
                        None => std::future::pending::<Option<_>>().await,
                    }
                } => {
                    self.ingest_event(ev);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_emits_line_when_buffer_non_empty() {
        let mut app = App::new(None);
        app.input = "hello".into();
        let out = app.handle_enter();
        assert_eq!(out.as_deref(), Some("hello"));
        assert!(app.input.is_empty());
        assert!(app.log.iter().any(|l| l.contains("hello")));
    }

    #[test]
    fn colon_q_quits_and_emits_nothing() {
        let mut app = App::new(None);
        app.input = ":q".into();
        let out = app.handle_enter();
        assert!(out.is_none());
        assert!(app.should_quit);
    }

    #[test]
    fn assistant_delta_accumulates_then_logs_on_turn() {
        let mut app = App::new(None);
        app.ingest_event(OrchEvent::AssistantDelta {
            session: deck_core::SessionId::new(),
            text: "hel".into(),
        });
        app.ingest_event(OrchEvent::AssistantDelta {
            session: deck_core::SessionId::new(),
            text: "lo".into(),
        });
        app.ingest_event(OrchEvent::AssistantTurn {
            session: deck_core::SessionId::new(),
            message: deck_core::Message {
                role: deck_core::Role::Assistant,
                content: "hello".into(),
                tool_calls: vec![],
            },
        });
        assert!(app.log.iter().any(|l| l.contains("hello")));
    }
}
