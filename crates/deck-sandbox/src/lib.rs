//! deck-sandbox — the ICE.
//!
//! Wraps a child process (an MCP server) with a seccomp BPF filter and a
//! landlock filesystem ruleset. On non-Linux targets, this crate degrades
//! to a no-op stub so the workspace still builds, but [`enforces`] reports
//! `false` and `--sandbox-strict` will refuse to launch untrusted servers.
//!
//! This is the *one* feature that distinguishes ono-sendai from every
//! other LLM TUI on GitHub: you can run an untrusted MCP server and trust
//! that, at worst, it can only touch the paths you whitelisted.

use deck_core::Sandbox;

pub mod profile;
pub use profile::SandboxProfile;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod stub;

#[cfg(target_os = "linux")]
pub use linux::LinuxSandbox as PlatformSandbox;
#[cfg(not(target_os = "linux"))]
pub use stub::StubSandbox as PlatformSandbox;

/// Short human-readable tag for diagnostics (`doctor` subcommand).
#[must_use]
pub fn availability() -> &'static str {
    PlatformSandbox::default().availability()
}

/// `true` if the host kernel actually enforces a policy when we apply one.
#[must_use]
pub fn enforces() -> bool {
    PlatformSandbox::default().enforces()
}
