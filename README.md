# ono-sendai

> Console Cowboy deck — a single-binary terminal cyberdeck.

`ono-sendai` is an offline-first TUI agent platform written in Rust. It pairs a
local LLM backend (Ollama HTTP — `llama.cpp` in 0.2) with an MCP client over
stdio JSON-RPC. The design pivots around a first-class `deck-sandbox` crate
that will (in 0.2) launch every untrusted MCP server through a seccomp BPF
filter + landlock filesystem ruleset; in `0.1` the sandbox crate ships the
trait surface and the policy types, but the fork-exec helper that actually
applies the filter is not yet wired (`Sandbox::enforces()` reports `false`).
Persistence is local SQLite; `age` encryption-at-rest lands in 0.2 alongside
the XDG `decks/<id>/identity.age` layout.

The name is a nod to the Ono-Sendai Cyberspace 7, the deck Case rides in
*Neuromancer*. This project is the deck — not the matrix. Not affiliated
with, endorsed by, or sponsored by William Gibson or his estate; the name
is used as homage to a fictional brand from the book.

## Status

Pre-alpha. Workspace skeleton + Phase-2 plumbing. Honest summary of what is
real today vs. what is on the roadmap lives in
[`CHANGELOG.md`](CHANGELOG.md) under *Known limitations of 0.1* and in
[`docs/ROADMAP.md`](docs/ROADMAP.md).

## Try it (no LLM daemon required)

The mock backend streams a deterministic echo, so a fresh checkout boots
the full TUI without Ollama:

```
cargo run -- --backend mock run
```

(`--release` works too but the debug build starts faster on a cold
checkout. Requires an interactive TTY — running through a pipe, CI
runner, or non-PTY ssh session will exit early with a clear message.)

Type a line, hit Enter, watch the mock reply stream in. `:q` exits.

## Why another LLM TUI?

A few similar projects already ship pieces of what `ono-sendai` aims at:

| Project                          | What it does today                                   | What ono-sendai bets on differently               |
|----------------------------------|------------------------------------------------------|---------------------------------------------------|
| Claude Code / Cursor             | Cloud-bound, no air-gap mode                         | offline-first                                     |
| aider                            | Git-centric CLI, no MCP host                         | MCP host + non-git workflows                      |
| gptme                            | Python CLI, MCP + sandboxed shell exec               | Rust single binary, no Python install required    |
| OpenCode                         | Go TUI, MCP + Ollama backend                         | exposes the sandbox crate as a reusable library   |
| OpenAI Codex CLI                 | Rust, seccomp + Landlock default-on (Linux)          | open workspace, plugin host, MCP-as-tools layer   |
| Continue / Codeium               | IDE plugin, not a deck                               | terminal-native, no editor coupling               |

Honest framing: as of 0.1 the headline feature — sandbox enforcement of
untrusted MCP tool execution — is **scaffolded but not yet wired**
(`Sandbox::enforces() == false`). OpenAI Codex already ships seccomp +
Landlock default-on on Linux. The differentiator ono-sendai is reaching
for is *first-class workspace decomposition*: `deck-sandbox`,
`deck-mcp`, `deck-llm` are reusable library crates, not private
internals. If you want to embed an MCP host with a sandbox policy in
your own Rust application, the goal is that you can depend on those
crates directly. The standalone binary is one consumer of the workspace,
not the whole project.

In 0.2 the fork+exec helper lands and `deck-sandbox` actually applies
the seccomp BPF + Landlock ruleset pre-`exec(2)`.

## Architecture

```
ratatui ─key─▶ deck-tui ─Cmd─▶ deck-orchestrator (mpsc hub)
                                     │ │ │ │ │
                                     ▼ ▼ ▼ ▼ ▼
                                deck-llm  deck-mcp  deck-store
                                deck-sandbox  deck-plugin
```

Eight crates in one Cargo workspace, two opt-in plugins (`icebreaker` for
red-team tooling, `mesh` for deck-to-deck via iroh — both shipped behind
features in `0.2`+).

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full picture.

## Build

```
cargo build --release
```

The build is a single static-ish Linux binary (musl target recommended for
distribution). macOS support is best-effort in `0.1`; sandbox features
degrade gracefully on platforms without seccomp/landlock.

## License

Dual-licensed under either of [MIT](LICENSE) or
[MIT](LICENSE-APACHE), at your option. This matches Rust ecosystem
convention; the Apache half gives downstream consumers an explicit patent
grant. All major dependencies are MIT / MIT / BSD; we deliberately
avoid GPL-3 and CC-BY-NC components (enforced by `deny.toml` in CI).

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion shall be dual-licensed as above, without any
additional terms or conditions.
