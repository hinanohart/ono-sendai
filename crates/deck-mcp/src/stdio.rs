//! A stdio-transport MCP client. Each instance owns one child process and
//! its stdin/stdout pipes. Reads and writes are serialized through Mutexes
//! to keep request/response ordering deterministic.

use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use deck_core::traits::ToolDescriptor;
use deck_core::{DeckError, McpClient, Result, ToolCall, ToolResult};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::wire::{make_request, JsonRpcResponse};

pub struct StdioMcpClient {
    name: String,
    next_id: AtomicU64,
    inner: Mutex<Inner>,
}

struct Inner {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl std::fmt::Debug for StdioMcpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StdioMcpClient")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl StdioMcpClient {
    pub async fn spawn(name: impl Into<String>, command: &str, args: &[String]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| DeckError::Mcp(format!("spawn {command}: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| DeckError::Mcp("no stdin pipe".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| DeckError::Mcp("no stdout pipe".into()))?;
        let me = Self {
            name: name.into(),
            next_id: AtomicU64::new(1),
            inner: Mutex::new(Inner {
                _child: child,
                stdin,
                stdout: BufReader::new(stdout),
            }),
        };
        me.initialize().await?;
        Ok(me)
    }

    async fn initialize(&self) -> Result<()> {
        let resp = self
            .request(
                "initialize",
                Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "ono-sendai", "version": env!("CARGO_PKG_VERSION")}
                })),
            )
            .await?;
        debug!(server = %self.name, ?resp, "mcp initialized");
        Ok(())
    }

    async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<JsonRpcResponse> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let line = make_request(id, method, params);
        let mut inner = self.inner.lock().await;
        inner
            .stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| DeckError::Mcp(format!("write: {e}")))?;
        inner
            .stdin
            .write_all(b"\n")
            .await
            .map_err(|e| DeckError::Mcp(format!("write newline: {e}")))?;
        inner
            .stdin
            .flush()
            .await
            .map_err(|e| DeckError::Mcp(format!("flush: {e}")))?;
        let mut buf = String::new();
        let n = inner
            .stdout
            .read_line(&mut buf)
            .await
            .map_err(|e| DeckError::Mcp(format!("read: {e}")))?;
        if n == 0 {
            return Err(DeckError::Mcp("server closed pipe".into()));
        }
        let resp: JsonRpcResponse = serde_json::from_str(buf.trim())?;
        if let Some(err) = &resp.error {
            warn!(code = err.code, msg = %err.message, "mcp jsonrpc error");
            return Err(DeckError::Mcp(format!(
                "rpc error {}: {}",
                err.code, err.message
            )));
        }
        Ok(resp)
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    fn server_name(&self) -> &str {
        &self.name
    }

    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>> {
        let resp = self.request("tools/list", None).await?;
        let result = resp
            .result
            .ok_or_else(|| DeckError::Mcp("missing result".into()))?;
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(tools
            .into_iter()
            .filter_map(|t| {
                let name = t.get("name")?.as_str()?.to_owned();
                let description = t
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                let schema = t.get("inputSchema").cloned().unwrap_or(json!({}));
                Some(ToolDescriptor {
                    name,
                    description,
                    json_schema: schema,
                })
            })
            .collect())
    }

    async fn call(&self, call: &ToolCall) -> Result<ToolResult> {
        let resp = self
            .request(
                "tools/call",
                Some(json!({
                    "name": call.tool,
                    "arguments": call.arguments,
                })),
            )
            .await?;
        let result = resp.result.unwrap_or(json!({}));
        let is_error = result
            .get("isError")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        Ok(ToolResult {
            call_id: call.id.clone(),
            content: result,
            is_error,
        })
    }
}
