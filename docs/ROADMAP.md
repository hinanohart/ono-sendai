# Roadmap

## 0.1 — Skeleton + MVP plumbing (this release)
- [x] Cargo workspace, 8 crates + 2 plugins.
- [x] Trait surface in `deck-core`.
- [x] Mock + Ollama LLM backends.
- [x] JSON-RPC stdio MCP client.
- [x] SQLite session store (plaintext).
- [x] TUI ↔ Orchestrator wired end-to-end.
- [x] CI green: fmt + clippy(-D warnings) + build + test + cargo-deny.
- [x] License hygiene enforced.

## 0.2 — Make the sandbox real
- [ ] `deck-sandbox`: fork+exec helper that applies seccomp BPF +
      landlock ruleset before `exec()`.
- [ ] Runtime probe (`landlock_create_ruleset(NULL, 0, ...)`) so
      `Sandbox::enforces()` is honest on old kernels.
- [ ] `deck-store`: age encrypt-on-write / decrypt-to-tmpfs lifecycle.
- [ ] `deck-llm`: `llama-cpp` backend behind a `llama-cpp` feature.
- [ ] `deck-mcp`: 30s `ApprovalRequest` popup in the TUI.
- [ ] `deck-plugin`: `wasmtime` loader.
- [ ] `ICEBREAKER` plugin: real subprocess bridge to `promptfoo` /
      `garak` / `PyRIT` (sandboxed).

## 1.0 — First user-visible cut
- [ ] macOS sandbox shim (sandbox_init / Endpoint Security where
      feasible; otherwise `Sandbox::enforces()` returns false and the
      UI warns).
- [ ] `--strict` mode that refuses to launch a non-sandboxed server.
- [ ] Onboarding flow (first launch → "create your first deck").
- [ ] Homebrew formula PR + cargo-binstall + AUR + `curl | sh`.

## 1.5 — Mesh
- [ ] `mesh` plugin: iroh QUIC hole-punch + automerge CRDT for
      deck-to-deck session replication.
- [ ] Demand validation first (issue + survey), then code.
