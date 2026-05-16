//! In-memory mock backend used for tests, offline demos, and the TUI when
//! no real LLM endpoint is reachable. Deterministic; never touches the
//! network.

use async_trait::async_trait;
use deck_core::{LlmBackend, Message, Result, Role};
use futures::stream::{self, BoxStream, StreamExt};

#[derive(Debug, Default, Clone)]
pub struct MockBackend {
    pub reply_template: String,
}

impl MockBackend {
    #[must_use]
    pub fn new(reply_template: impl Into<String>) -> Self {
        Self {
            reply_template: reply_template.into(),
        }
    }

    fn shape_reply(&self, messages: &[Message]) -> String {
        let last_user = messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, Role::User))
            .map_or("(no input)", |m| m.content.as_str());
        if self.reply_template.is_empty() {
            format!("[mock] you said: {last_user}")
        } else {
            self.reply_template.replace("{input}", last_user)
        }
    }
}

#[async_trait]
impl LlmBackend for MockBackend {
    fn id(&self) -> String {
        "mock@in-process".into()
    }

    async fn complete(&self, _model: &str, messages: &[Message]) -> Result<Message> {
        Ok(Message {
            role: Role::Assistant,
            content: self.shape_reply(messages),
            tool_calls: vec![],
        })
    }

    async fn stream(
        &self,
        _model: &str,
        messages: &[Message],
    ) -> Result<BoxStream<'static, Result<Message>>> {
        let reply = self.shape_reply(messages);
        let chunks: Vec<Result<Message>> = reply
            .split_inclusive(' ')
            .map(|c| {
                Ok(Message {
                    role: Role::Assistant,
                    content: c.to_owned(),
                    tool_calls: vec![],
                })
            })
            .collect();
        Ok(stream::iter(chunks).boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn mock_complete_echoes_last_user() {
        let m = MockBackend::default();
        let msgs = vec![Message {
            role: Role::User,
            content: "hi".into(),
            tool_calls: vec![],
        }];
        let reply = m.complete("ignored", &msgs).await.unwrap();
        assert!(reply.content.contains("hi"));
    }

    #[tokio::test]
    async fn mock_stream_emits_chunks() {
        let m = MockBackend::default();
        let msgs = vec![Message {
            role: Role::User,
            content: "a b c".into(),
            tool_calls: vec![],
        }];
        let mut s = m.stream("ignored", &msgs).await.unwrap();
        let mut full = String::new();
        while let Some(c) = s.next().await {
            full.push_str(&c.unwrap().content);
        }
        assert!(full.contains("a b c"));
    }
}
