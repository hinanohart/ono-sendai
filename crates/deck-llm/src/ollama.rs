//! Ollama HTTP backend. Talks to a local daemon at `http://127.0.0.1:11434`
//! by default. Streaming is line-delimited JSON over a POST to `/api/chat`.
//!
//! The streaming decoder buffers TCP chunks and splits on `\n` before
//! attempting to parse each line — Ollama can pack multiple JSON objects
//! into one TCP chunk, or split one object across two chunks, and a naïve
//! `serde_json::from_str` on each `bytes_stream()` item silently corrupts.

use std::time::Duration;

use async_stream::try_stream;
use async_trait::async_trait;
use deck_core::{DeckError, LlmBackend, Message, Result, Role};
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct OllamaBackend {
    endpoint: String,
    client: Client,
}

impl OllamaBackend {
    /// Build an Ollama client. Returns an error if reqwest fails to
    /// initialize (broken system roots, missing entropy, etc.) so a
    /// transient TLS-init failure does not become a process abort.
    pub fn new(endpoint: String, timeout: Duration) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| DeckError::Llm(format!("reqwest client init: {e}")))?;
        Ok(Self { endpoint, client })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<WireMessage>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct WireMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: WireMessage,
    #[serde(default)]
    done: bool,
}

fn to_wire(m: &Message) -> WireMessage {
    WireMessage {
        role: m.role.as_wire_str().into(),
        content: m.content.clone(),
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    fn id(&self) -> String {
        format!("ollama@{}", self.endpoint)
    }

    async fn complete(&self, model: &str, messages: &[Message]) -> Result<Message> {
        let body = ChatRequest {
            model,
            messages: messages.iter().map(to_wire).collect(),
            stream: false,
        };
        let url = format!("{}/api/chat", self.endpoint);
        debug!(%url, model, "ollama complete");
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DeckError::Llm(e.to_string()))?
            .error_for_status()
            .map_err(|e| DeckError::Llm(e.to_string()))?
            .json::<ChatResponse>()
            .await
            .map_err(|e| DeckError::Llm(e.to_string()))?;
        Ok(Message {
            role: Role::Assistant,
            content: resp.message.content,
            tool_calls: vec![],
        })
    }

    async fn stream(
        &self,
        model: &str,
        messages: &[Message],
    ) -> Result<BoxStream<'static, Result<Message>>> {
        let body = ChatRequest {
            model,
            messages: messages.iter().map(to_wire).collect(),
            stream: true,
        };
        let url = format!("{}/api/chat", self.endpoint);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DeckError::Llm(e.to_string()))?
            .error_for_status()
            .map_err(|e| DeckError::Llm(e.to_string()))?;

        let s = try_stream! {
            let mut bytes = resp.bytes_stream();
            let mut buf = String::new();
            while let Some(chunk) = bytes.next().await {
                let chunk = chunk.map_err(|e| DeckError::Llm(e.to_string()))?;
                let s = std::str::from_utf8(&chunk)
                    .map_err(|e| DeckError::Llm(format!("non-utf8 chunk: {e}")))?;
                buf.push_str(s);
                while let Some(nl) = buf.find('\n') {
                    let line: String = buf.drain(..=nl).collect();
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let parsed: ChatResponse = serde_json::from_str(trimmed)?;
                    yield Message {
                        role: Role::Assistant,
                        content: parsed.message.content,
                        tool_calls: vec![],
                    };
                    if parsed.done {
                        return;
                    }
                }
            }
            let tail = buf.trim();
            if !tail.is_empty() {
                let parsed: ChatResponse = serde_json::from_str(tail)?;
                yield Message {
                    role: Role::Assistant,
                    content: parsed.message.content,
                    tool_calls: vec![],
                };
            }
        };
        Ok(s.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_includes_endpoint() {
        let b = OllamaBackend::new("http://localhost:11434".into(), Duration::from_secs(10))
            .expect("client init in test");
        assert!(b.id().contains("11434"));
    }
}
