//! Ollama HTTP backend. Talks to a local daemon at `http://127.0.0.1:11434`
//! by default. Streaming is line-delimited JSON over a POST to `/api/chat`.

use std::time::Duration;

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
    #[must_use]
    pub fn new(endpoint: String, timeout: Duration) -> Self {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client init");
        Self { endpoint, client }
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
    #[allow(dead_code)]
    #[serde(default)]
    done: bool,
}

const fn wire_role(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

fn to_wire(m: &Message) -> WireMessage {
    WireMessage {
        role: wire_role(m.role).into(),
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
        let byte_stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk.map_err(|e| DeckError::Llm(e.to_string()))?;
            let line = std::str::from_utf8(&chunk)
                .map_err(|e| DeckError::Llm(format!("non-utf8 chunk: {e}")))?
                .trim()
                .to_owned();
            if line.is_empty() {
                return Ok(Message {
                    role: Role::Assistant,
                    content: String::new(),
                    tool_calls: vec![],
                });
            }
            let parsed: ChatResponse = serde_json::from_str(&line)?;
            Ok(Message {
                role: Role::Assistant,
                content: parsed.message.content,
                tool_calls: vec![],
            })
        });
        Ok(byte_stream.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_includes_endpoint() {
        let b = OllamaBackend::new("http://localhost:11434".into(), Duration::from_secs(10));
        assert!(b.id().contains("11434"));
    }
}
