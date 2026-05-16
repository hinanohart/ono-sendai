//! User-facing configuration. Resolved from (precedence high → low):
//!   1. CLI flags
//!   2. environment variables prefixed with `ONOSENDAI_`
//!   3. TOML config file (`$XDG_CONFIG_HOME/ono-sendai/config.toml`)
//!   4. compiled-in defaults
//!
//! The resolver lives in the binary crate; this module only declares the
//! shape so that every other crate can reason about it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub llm: LlmConfig,
    pub mcp: McpConfig,
    pub store: StoreConfig,
    pub sandbox: SandboxConfig,
    pub tui: TuiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Backend selector: "ollama" or "llama-cpp".
    pub backend: String,
    /// Endpoint URL (HTTP). Ignored for `llama-cpp` (which is in-process).
    pub endpoint: String,
    /// Default model name.
    pub model: String,
    /// Optional per-request timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            backend: "ollama".into(),
            endpoint: "http://127.0.0.1:11434".into(),
            model: "llama3.1".into(),
            timeout_secs: 120,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    /// Declared MCP servers. Each launches as a child process (stdio transport).
    #[serde(default)]
    pub servers: Vec<McpServerSpec>,
    /// Approval popup timeout. 0 = require explicit approval, no auto-deny.
    #[serde(default = "default_approval_timeout")]
    pub approval_timeout_secs: u64,
}

const fn default_approval_timeout() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerSpec {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// If `true`, launch this server through `deck-sandbox`.
    #[serde(default = "default_true")]
    pub sandbox: bool,
    /// Optional path to a seccomp/landlock profile.
    pub profile: Option<PathBuf>,
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoreConfig {
    /// Root directory for encrypted decks (default: `$XDG_DATA_HOME/ono-sendai/decks`).
    pub root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// If `true`, refuse to run an MCP server whose `sandbox` flag is false
    /// on platforms that support sandboxing. Defaults to true.
    pub strict: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self { strict: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Frame tick in milliseconds. 16ms ≈ 60fps.
    pub tick_ms: u64,
    /// Mouse capture toggle.
    pub mouse: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            tick_ms: 16,
            mouse: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip_through_toml() {
        let cfg = Config::default();
        let s = toml::to_string(&cfg).expect("serialize default config to toml");
        let back: Config = toml::from_str(&s).expect("parse back default config");
        assert_eq!(back.llm.backend, cfg.llm.backend);
        assert_eq!(back.tui.tick_ms, cfg.tui.tick_ms);
    }
}
