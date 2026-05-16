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
*Neuromancer*. This project is the deck — not the matrix.

## Status

Pre-alpha. Workspace skeleton + Phase-2 plumbing. Honest summary of what is
real today vs. what is on the roadmap lives in
[`CHANGELOG.md`](CHANGELOG.md) under *Known limitations of 0.1* and in
[`docs/ROADMAP.md`](docs/ROADMAP.md).

## Try it (no LLM daemon required)

The mock backend streams a deterministic echo, so a fresh checkout boots
the full TUI without Ollama:

```
cargo run --release -- --backend mock run
```

Type a line, hit Enter, watch the mock reply stream in. `:q` exits.

## Why another LLM TUI?

| Existing                   | Gap                                                |
|----------------------------|----------------------------------------------------|
| Claude Code / Cursor       | Cloud-bound, no air-gap mode                       |
| aider                      | Git-centric, no MCP host                           |
| Continue, Codeium          | IDE plugin, not a deck                             |
| Ollama UIs (Web)           | Browser-bound, no sandbox for tool execution       |
| **ono-sendai**             | **TUI + local LLM + MCP + sandboxed tool host**    |

The planned differentiator is the *sandbox*: in 0.2, every MCP tool call
from an untrusted server will be dispatched through `deck-sandbox`
(seccomp BPF + landlock filesystem ruleset) so a malicious or poisoned MCP
server cannot exfiltrate beyond its declared filesystem ruleset. In 0.1
the crate ships the trait surface and the `SandboxProfile` types; the
fork+exec helper that applies the policy pre-`exec(2)` is the headline
work item of the next release.

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

MIT. All major dependencies are MIT / Apache-2.0 / BSD; we deliberately avoid
GPL-3 and CC-BY-NC components.
