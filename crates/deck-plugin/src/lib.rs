//! deck-plugin — WASM plugin host surface.
//!
//! The trait + manifest format are stabilized in `0.1`; the actual
//! `wasmtime` loader lands in `0.2`. We keep the SDK dep out of `0.1` so
//! first-time builds stay under a minute.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub entry: PathBuf,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Plugin may register one or more MCP tools.
    McpTool,
    /// Plugin may render a panel in the TUI.
    TuiPanel,
    /// Plugin may receive orchestrator events read-only.
    EventConsumer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses_minimal_toml() {
        let s = r#"
            name = "icebreaker"
            version = "0.1.0"
            entry = "icebreaker.wasm"
            capabilities = ["mcp-tool"]
            description = "promptfoo+garak+PyRIT red-team bundle"
        "#;
        let m: PluginManifest = toml::from_str(s).expect("parse");
        assert_eq!(m.name, "icebreaker");
        assert_eq!(m.capabilities, vec![Capability::McpTool]);
    }
}
