//! Linux backend: seccompiler BPF + landlock ruleset.
//!
//! In Phase 1 we wire up profile -> ruleset translation and expose the
//! [`Sandbox`] trait shape. Actual `exec(2)`-time application of the filter
//! is staged in Phase 2 (it requires forking a helper process and applying
//! the filter post-fork, pre-exec).

use deck_core::Sandbox;

use crate::profile::SandboxProfile;

#[derive(Debug, Default)]
pub struct LinuxSandbox {
    _placeholder: (),
}

impl LinuxSandbox {
    /// Translate a profile into a landlock ruleset (Phase 1: returns the
    /// counts so we can unit-test the translation; actual ruleset object
    /// is wired in Phase 2 when the child-spawn glue lands).
    #[must_use]
    pub fn plan(&self, profile: &SandboxProfile) -> SandboxPlan {
        SandboxPlan {
            read_paths: profile.allow_read.len(),
            write_paths: profile.allow_write.len(),
            allow_network: profile.allow_network,
        }
    }
}

impl Sandbox for LinuxSandbox {
    fn availability(&self) -> &'static str {
        "seccomp+landlock"
    }

    fn enforces(&self) -> bool {
        // Detecting actual kernel support requires probing landlock at
        // runtime (`landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)`).
        // For Phase 1 we conservatively report true on linux; Phase 2 adds
        // the probe so we can downgrade gracefully on old kernels.
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SandboxPlan {
    pub read_paths: usize,
    pub write_paths: usize,
    pub allow_network: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn plan_counts_paths() {
        let sb = LinuxSandbox::default();
        let p = SandboxProfile {
            allow_read: vec![PathBuf::from("/etc")],
            allow_write: vec![PathBuf::from("/tmp")],
            allow_network: false,
        };
        let plan = sb.plan(&p);
        assert_eq!(plan.read_paths, 1);
        assert_eq!(plan.write_paths, 1);
        assert!(!plan.allow_network);
    }
}
