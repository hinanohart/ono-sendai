//! JSON-RPC 2.0 wire types for MCP. Only the subset we use.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest<'a> {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

#[must_use]
pub fn make_request(id: u64, method: &str, params: Option<Value>) -> String {
    let req = JsonRpcRequest {
        jsonrpc: "2.0",
        id,
        method,
        params,
    };
    serde_json::to_string(&req).expect("serialize jsonrpc")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_with_jsonrpc_field() {
        let s = make_request(
            1,
            "initialize",
            Some(json!({"protocolVersion": "2024-11-05"})),
        );
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("\"method\":\"initialize\""));
    }
}
