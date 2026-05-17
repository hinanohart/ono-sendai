# Changelog

All notable changes to this project will be documented in this file. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1] - 2026-05-18

> **Note**: `v0.1.0` was tagged *before* the post-publication audit
> passes. It boots the TUI with several silent-corruption / desync /
> teardown bugs. Please use `0.1.1` or newer.

### Fixed (round 3 audit pass)
- **TUI now refuses non-TTY stdin with a clear message** instead of
  failing inside crossterm with cryptic "No such device" and an
  exit-0 false-success path. Running through a pipe, CI runner, or
  non-PTY ssh now exits with a non-zero status and a one-line hint.
- **`TerminalGuard` (RAII)** restores raw-mode + alt-screen on any
  exit path, including panics and errors during `App::run`. Previous
  shape leaked raw-mode if `app.run()` returned `Err`.
- **MCP stdio client now pairs response id to request id.** An
  unsolicited server notification or out-of-order reply previously
  would have been returned to the wrong caller; now it poisons the
  client (same behavior as the existing timeout-desync guard).
- **`SqliteStore::append` wraps the INSERT-session + SELECT MAX(seq) +
  INSERT-message sequence in a transaction.** Concurrent appends on
  the same session cannot collide on the `(session_id, seq)` primary
  key anymore. Mutex `.expect("store mutex")` panics in all three
  store paths are replaced with `DeckError::Store("mutex poisoned: …")`.
- **`Handle::submit` returns `deck_core::Result<()>`** instead of
  leaking `tokio::sync::mpsc::error::SendError<Command>` into the
  public API. The channel implementation is now an internal detail.
- **`init_tracing` `EnvFilter` directive expanded** from the broken
  `deck_=` (matched no crate) to per-crate `deck_core=…,deck_llm=…`
  so `-vv` / `-vvv` actually raise log verbosity for the workspace.
- **Dual-licensed MIT OR Apache-2.0** (was MIT-only). Apache half
  supplies an explicit patent grant for downstream consumers; matches
  Rust ecosystem convention. `LICENSE-APACHE` added.

### Fixed (post-publication audit pass — included in 0.1.1)
- Ollama streaming decoder now buffers TCP chunks and splits on `\n`
  before parsing; the previous version called `serde_json::from_str`
  on each raw chunk and silently corrupted multi-line / split-line
  responses.
- `SqliteStore::load` errors on unknown role strings and on malformed
  `tool_calls` JSON instead of silently coercing to `Assistant` /
  empty `Vec`.
- `OllamaBackend::new` is fallible: a reqwest init failure (broken
  system roots, missing entropy) returns `DeckError` instead of
  panicking.
- `EventStream` no longer emits a synthetic `Tick` on every unhandled
  key (`map_key` returns `Option<Event>`); the poll thread has an
  `AtomicBool` shutdown flag and is joined on drop.
- `deck-orchestrator` `run_loop` spawns one task per `UserMessage`
  command in a `JoinSet` and drains it on shutdown so a slow LLM
  stream cannot head-of-line block other sessions, and so writes to
  `store` no longer race against process exit.
- `ApproveTool` / `DenyTool` commands now emit an `Event::Error`
  explaining the feature is not yet wired in 0.1, instead of silently
  warning into the log.
- MCP stdio client wraps each `read_line` in a 30-second
  `tokio::time::timeout` and sets `poisoned: AtomicBool` on timeout so
  a half-broken wire cannot be silently reused.
- `Role` gains canonical `as_wire_str` / `from_wire_str` helpers in
  `deck-core`; LLM wire conversion and store encode/decode now share
  one source of truth.
- `--config` flag actually loads a TOML file via figment with
  precedence `env > cli > $XDG_CONFIG_HOME/ono-sendai/config.toml >
  defaults` instead of being silently ignored.
- `doctor` reports the real `enforces` value (currently `false` on
  every platform) and prefixes the store path with "plaintext SQLite"
  so we stop advertising encryption we have not shipped.

### Doc + community
- README hero paragraph no longer claims age encryption or a wired
  sandbox in 0.1; both are explicitly framed as 0.2 work.
- README *Why another LLM TUI?* table now compares honestly against
  gptme, OpenCode, OpenAI Codex CLI — competing projects already ship
  pieces of what 0.1 aims at, and the table reflects that.
- "Not affiliated with William Gibson or his estate" disclaimer added.
- `docs/ARCHITECTURE.md` diagram annotates `age 0.2` / `enforce 0.2`.
- `deck-store` package description rewritten to "SQLite session store
  (age encryption planned for 0.2)".
- `SECURITY.md` describes the 0.1 sandbox honesty boundary.
- `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), GitHub issue
  templates (bug + feature), pull-request template, and a `cargo` +
  `github-actions` `dependabot.yml` are now in `.github/`.
- `plugins/icebreaker` and `plugins/mesh` are marked `publish = false`
  so a workspace-wide `cargo publish` does not try to push them.

## [0.1.0] - 2026-05-16 (superseded by 0.1.1)

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

[Unreleased]: https://github.com/hinanohart/ono-sendai/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/hinanohart/ono-sendai/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/hinanohart/ono-sendai/releases/tag/v0.1.0
