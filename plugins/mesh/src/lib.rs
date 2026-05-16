//! mesh — deck-to-deck plugin (iroh + automerge).
//!
//! Demand-validated externally before any real implementation lands; this
//! crate is reserved for `1.5` (iroh QUIC hole-punch + CRDT session
//! replication). Phase 1 only stakes the manifest.

use deck_plugin::{Capability, PluginManifest};
use std::path::PathBuf;

#[must_use]
pub fn manifest() -> PluginManifest {
    PluginManifest {
        name: "mesh".into(),
        version: "0.0.1".into(),
        entry: PathBuf::from("mesh.wasm"),
        capabilities: vec![Capability::EventConsumer],
        description: "Deck-to-deck session replication via iroh + automerge (reserved for 1.5)"
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_event_consumer_only_for_now() {
        let m = manifest();
        assert_eq!(m.capabilities, vec![Capability::EventConsumer]);
    }
}
