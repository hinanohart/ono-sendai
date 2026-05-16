# Security policy

`ono-sendai`'s pitch is to give users a way to run untrusted Model Context
Protocol (MCP) servers without surrendering their machine. That promise
will not land in 0.1 — the seccomp + landlock enforcement layer is
scaffolded but not wired (`Sandbox::enforces()` returns `false`). Until
0.2 ships, *do not assume MCP servers launched by ono-sendai are
sandboxed*.

## Reporting a vulnerability

If you discover a security issue:

- For **non-critical** bugs (e.g. the TUI shows a server-supplied string
  without escaping, a CLI flag mishandles a path), open a regular GitHub
  issue and prefix the title with `[security]`.
- For **critical** issues (sandbox escape ideas once 0.2 is wired,
  authentication / credential exposure, code execution paths), do **not**
  open a public issue. Email the maintainer privately. The email lives
  in the GitHub profile of the repo owner (currently
  https://github.com/hinanohart). PGP not required.

We will acknowledge within 7 days and aim for a patched release within
30 days of acknowledgement.

## Out of scope

- Bugs in upstream crates (file them with the upstream — ratatui,
  rusqlite, age, seccompiler, landlock, etc.).
- Local privilege escalation requiring root access to the same machine
  the user already trusts.
- Denial of service against the user's own LLM (just stop typing).

## Honest scope of 0.1

The sandbox is the differentiator. In 0.1, the sandbox is:

- ✅ A real crate (`deck-sandbox`) with a final trait surface.
- ✅ Wires `seccompiler` (Linux) and `landlock` (Linux) as `cfg(linux)`
  dependencies.
- ✅ Holds a `SandboxProfile` type with read / write path allow lists.
- ❌ Does **not** apply any filter or ruleset to a child process.
- ❌ The MCP `StdioMcpClient::spawn()` path calls `Command::new()`
  directly, without sandbox involvement.

This is documented in `CHANGELOG.md` under *Known limitations of 0.1* and
in `docs/ROADMAP.md` under *0.2 — Make the sandbox real*. Treat any
contrary claim in the documentation as a bug and file it.
