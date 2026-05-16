//! icebreaker — red-team plugin.
//!
//! Bridges to external tooling (`promptfoo`, `garak`, `PyRIT`) by spawning
//! them as subprocesses through the [`deck_sandbox`] boundary. Phase 1
//! only ships the manifest + capability declaration; subprocess wiring
//! lands in 0.2.

use deck_plugin::{Capability, PluginManifest};
use std::path::PathBuf;

#[must_use]
pub fn manifest() -> PluginManifest {
    PluginManifest {
        name: "icebreaker".into(),
        version: "0.1.0".into(),
        entry: PathBuf::from("icebreaker.wasm"),
        capabilities: vec![Capability::McpTool, Capability::TuiPanel],
        description: "Red-team bundle: promptfoo + garak + PyRIT bridges".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declares_red_team_capabilities() {
        let m = manifest();
        assert_eq!(m.name, "icebreaker");
        assert!(m.capabilities.contains(&Capability::McpTool));
    }
}
