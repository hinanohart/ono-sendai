# Changelog

All notable changes to this project will be documented in this file. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Workspace skeleton: 8 crates (`deck-core`, `deck-llm`, `deck-mcp`,
  `deck-store`, `deck-tui`, `deck-sandbox`, `deck-orchestrator`,
  `deck-plugin`) and 2 plugins (`icebreaker`, `mesh`).
- `MockBackend` (offline / test) and `OllamaBackend` (HTTP).
- JSON-RPC 2.0 MCP client with stdio transport (`StdioMcpClient`).
- Persisted SQLite session store (`age` encryption arrives in 0.2).
- Sandbox crate with seccomp + landlock dependency wiring (full
  enforcement in 0.2; the trait surface is final).
- TUI with ratatui: title / log / input / status panes, 60fps tick,
  Ctrl-C / `:q` exit, AssistantDelta streaming.
- Orchestrator `Runtime`: tokio mpsc command channel, broadcast event
  channel, end-to-end user-turn → LLM stream → store append loop.
- CI: rustfmt + clippy (-D warnings, pedantic + nursery curated) +
  build + test on ubuntu + macos + cargo-deny (advisories / bans /
  licenses / sources).
- MIT license throughout; cargo-deny rejects GPL-3 / AGPL / CC-BY-NC.

### Known limitations of `0.1`
- The sandbox host wires seccomp + landlock dependencies but does not
  yet fork+exec a child with the policy applied — coming in `0.2`.
- MCP transport handles JSON-RPC stdio only (no rmcp SDK yet).
- The mesh plugin is reserved for `1.5`; iroh wiring is not built.
- `wasmtime` plugin loader will land in `0.2`; today `deck-plugin`
  only owns the manifest + capability types.

[Unreleased]: https://github.com/runza/ono-sendai/compare/v0.0.0...HEAD
