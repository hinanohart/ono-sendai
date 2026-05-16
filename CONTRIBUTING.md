# Contributing

Thanks for considering a patch. `ono-sendai` is in pre-alpha and the
surface area is changing fast, but issues and small PRs are welcome.

## Ground rules

- **License**: contributions are accepted under MIT. The project will
  not accept GPL-3, AGPL, or non-commercial licensed code; CI rejects
  such dependencies via `deny.toml`.
- **Style**: `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings`
  must be clean. CI enforces both.
- **Tests**: new code paths need a unit test. Integration tests live
  under `tests/`.
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
