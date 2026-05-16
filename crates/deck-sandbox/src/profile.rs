//! Sandbox profile description, decoupled from any kernel API.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxProfile {
    /// Paths the sandboxed process may read.
    #[serde(default)]
    pub allow_read: Vec<PathBuf>,
    /// Paths the sandboxed process may read+write.
    #[serde(default)]
    pub allow_write: Vec<PathBuf>,
    /// Whether the process may make outbound network calls.
    /// On linux we approximate this with a seccomp rule on `socket(2)`.
    #[serde(default)]
    pub allow_network: bool,
}
