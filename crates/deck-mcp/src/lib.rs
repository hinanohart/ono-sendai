//! deck-mcp — MCP client using JSON-RPC over stdio.
//!
//! Phase 1 implements the protocol surface directly (initialize, `list_tools`,
//! `call_tool`) without depending on the full `rmcp` SDK so we keep our
//! dependency graph slim and our wire format auditable. Phase 2 will swap
//! the transport layer for `rmcp` once its API stabilizes.

pub mod stdio;
pub mod wire;

pub use stdio::StdioMcpClient;
