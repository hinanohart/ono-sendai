# Architecture

`ono-sendai` is a Cargo workspace of eight library crates and two
plugins, all driven by a single binary that owns the TUI.

```
┌──────────────────────────────────────────────────────────┐
│ ratatui (deck-tui)                                       │
│   ↑ ↓ key/render events                                  │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ deck-orchestrator (mpsc command / broadcast event)  │ │
│  └─────────────────────────────────────────────────────┘ │
│       │            │           │             │           │
│  ┌────▼───┐  ┌─────▼───┐  ┌────▼────┐  ┌─────▼──────┐    │
│  │ deck-  │  │ deck-   │  │ deck-   │  │ deck-      │    │
│  │ llm    │  │ mcp     │  │ store   │  │ sandbox    │    │
│  │ (ollama│  │ (stdio  │  │ (sqlite,│  │ (seccomp + │    │
│  │ / mock)│  │ jsonrpc)│  │ age 0.2)│  │ landlock —  │    │
│  │        │  │         │  │         │  │  enforce 0.2)│  │
│  └────────┘  └─────────┘  └─────────┘  └────────────┘    │
│                                                          │
│              deck-plugin (wasmtime, 0.2)                 │
└──────────────────────────────────────────────────────────┘
                          ↑
            (0.2) external untrusted MCP servers
            will be launched **through** deck-sandbox
```

## Design pillars

### 1. Single static binary
The release profile (`lto = "thin"`, `codegen-units = 1`,
`panic = "abort"`, `strip = "symbols"`) targets a one-file
distribution. There is no Electron, no Python, no JavaScript runtime.

### 2. Offline-first
The default backend (Ollama HTTP) and `MockBackend` both run without
internet. The session store is local SQLite. There is no telemetry
and no analytics endpoint.

### 3. Sandbox is the differentiator
Every other LLM TUI lets MCP tool servers run with the full privileges
of the user. `ono-sendai` routes untrusted servers through
`deck-sandbox`, which (on Linux) applies a seccomp BPF filter plus a
landlock filesystem ruleset before `exec()`. The trait API is final;
the kernel-side enforcement is staged: trait + profile types in 0.1,
fork+exec helper + landlock probe in 0.2.

### 4. Trait-shaped at the boundaries
`deck-core` declares four traits: `LlmBackend`, `McpClient`, `Store`,
`Sandbox`. The orchestrator depends only on these. Swapping Ollama
for llama.cpp, swapping SQLite for an alternative store, or adding a
new sandbox backend is purely additive.

### 5. License-clean by enforcement
`deny.toml` blocks GPL-3 / AGPL / CC-BY-NC by whitelist (any license
not in `licenses.allow` fails CI). The accepted set is the standard
permissive surface: MIT, Apache-2.0, BSD-{2,3}-Clause, ISC, Unicode-3.0,
Zlib, CC0-1.0, MPL-2.0, OpenSSL, CDLA-Permissive-2.0.

## Process model

A single tokio runtime hosts the TUI render loop, the orchestrator
task, and one task per spawned MCP server. The TUI and any other
client interacts with the orchestrator through a clonable
`deck_orchestrator::Handle` (one bounded mpsc + one broadcast).
