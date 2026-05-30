# Contributing

Thanks for considering a patch. `ono-sendai` is in pre-alpha and the
surface area is changing fast, but issues and small PRs are welcome.

## Ground rules

- **License**: contributions are accepted under MIT OR Apache-2.0 (the project's dual license). The project will
  not accept GPL-3, AGPL, or non-commercial licensed code; CI rejects
  such dependencies via `deny.toml`.
- **Authorship**: published crates list `authors = ["ono-sendai
  contributors"]` as a collective identity. Individual authorship lives
  in the git history. We don't run DCO sign-off (yet).
- **Style**: `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings`
  must be clean. CI enforces both.
- **Tests**: new code paths need a unit test alongside the code (each
  `deck-*` crate keeps its own `#[cfg(test)] mod tests`). Workspace-
  level integration tests are not used in 0.1; if you need one, put it
  under the relevant crate's `tests/` directory and gate it with
  `#[cfg(feature = "integration")]`.
- **Commit messages**: imperative present-tense subject ≤72 chars, body
  optional. Reference issues by number, not by internal rule numbers.

## Local checks

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo deny check
```

All four must pass before a PR is ready for review.

## Repo layout

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
