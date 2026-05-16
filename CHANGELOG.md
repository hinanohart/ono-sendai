# Changelog

All notable changes to this project will be documented in this file. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed (post-publication audit pass)
- Ollama streaming decoder now buffers TCP chunks and splits on `\n`
  before parsing; the previous version called `serde_json::from_str`
  on each raw chunk and silently corrupted multi-line / split-line
  responses.
- `SqliteStore::load` now errors on unknown role strings and on
  malformed `tool_calls` JSON instead of silently coercing to
  `Assistant` / empty `Vec`.
- `OllamaBackend::new` is fallible: a reqwest init failure (broken
  system roots, missing entropy) returns `DeckError` instead of
  panicking.
- `EventStream` no longer emits a synthetic `Tick` on every unhandled
  key (`map_key` returns `Option<Event>`); the poll thread has an
  `AtomicBool` shutdown flag and is joined on drop.
- `deck-orchestrator` `run_loop` spawns one task per `UserMessage`
  command so a slow LLM stream on one session does not head-of-line
  block other sessions or future tool-approval commands.
- `ApproveTool` / `DenyTool` commands now emit an `Event::Error`
  explaining the feature is not yet wired in 0.1, instead of silently
  warning into the log.
- MCP stdio client wraps each `read_line` in a 30-second
  `tokio::time::timeout` so a hanging server cannot stall the
  orchestrator forever.
- `Role` gains canonical `as_wire_str` / `from_wire_str` helpers in
  `deck-core`; LLM wire conversion and store encode/decode now share
  one source of truth.
- `--config` flag actually loads a TOML file via figment with
  precedence `env > cli > $XDG_CONFIG_HOME/ono-sendai/config.toml >
  defaults` instead of being silently ignored.
- `doctor` reports the real `enforces` value (currently `false` on
  every platform) and prefixes the store path with "plaintext SQLite"
  so we stop advertising encryption we have not shipped.

### Doc fixes
- README hero paragraph no longer claims age encryption or a wired
  sandbox in 0.1; both are explicitly framed as 0.2 work.
- `docs/ARCHITECTURE.md` diagram annotates `age 0.2` / `enforce 0.2`.
- `deck-store` package description rewritten to "SQLite session store
  (age encryption planned for 0.2)".
- Added `SECURITY.md` describing the 0.1 sandbox honesty boundary and
  how to report.
- `CHANGELOG.md` Unreleased link points to `v0.1.0...HEAD` (was
  `v0.0.0...HEAD`, a dead link).

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

[Unreleased]: https://github.com/hinanohart/ono-sendai/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/hinanohart/ono-sendai/releases/tag/v0.1.0
