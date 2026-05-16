# ono-sendai

> Console Cowboy deck — a single-binary terminal cyberdeck.

`ono-sendai` is an offline-first TUI agent platform written in Rust. It pairs a
local LLM backend (ollama / llama.cpp) with a built-in MCP client and runs
untrusted MCP servers inside a seccomp + landlock sandbox. Encrypted context
lives entirely on disk via `age`; nothing leaves your machine unless you
explicitly wire a remote tool.

The name is a nod to the Ono-Sendai Cyberspace 7, the deck Case rides in
*Neuromancer*. This project is the deck — not the matrix.

## Status

Pre-alpha. Workspace skeleton + scaffolding only. See [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Why another LLM TUI?

| Existing                   | Gap                                                |
|----------------------------|----------------------------------------------------|
| Claude Code / Cursor       | Cloud-bound, no air-gap mode                       |
| aider                      | Git-centric, no MCP host                           |
| Continue, Codeium          | IDE plugin, not a deck                             |
| Ollama UIs (Web)           | Browser-bound, no sandbox for tool execution       |
| **ono-sendai**             | **TUI + local LLM + MCP + sandboxed tool host**    |

The differentiator is the *sandbox*: every MCP tool call from an untrusted
server is dispatched through `deck-sandbox` (seccomp BPF + landlock filesystem
ruleset), so a malicious or poisoned MCP server cannot exfiltrate beyond its
declared filesystem ruleset.

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
