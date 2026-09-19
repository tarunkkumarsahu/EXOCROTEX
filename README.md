# EXOCORTEX

A Rust-first experimental human cognitive-extension system. The human retains authority over consequential actions; models and tools will be replaceable adapters around a typed cognitive kernel.

## Working milestones

**V0.1** — Rust cognitive events, provenance types, action proposals, approval boundary and interactive CLI.

**V0.2** — local SQLite-backed memory store, four memory kinds, source/timestamp/epistemic tracking, correction/supersession, explicit-topic conflict flags, redaction, JSON export, standalone DB backups and restart tests.

V0.2 is **not yet an AI reasoning engine**: the CLI does not use an LLM, execute external tools, automatically verify factual truth or run scheduled reminders. V0.1 action proposals remain session-only.

## Start on Windows / PowerShell

Install stable Rust + MSVC build tools first, then inside the repository:

```powershell
cargo check --workspace
cargo test --workspace
cargo run -p exo-cli
```

First try:

```text
exo> :remember EXOCORTEX is our cognitive extension project.
exo> :recall EXOCORTEX
exo> :exit
```

Run `cargo run -p exo-cli` again and `:recall EXOCORTEX`. The record should still be there with its source event UUID and timestamp. To inspect the source, paste that source UUID into `:source <uuid>`.

Default DB: `%USERPROFILE%\.exocortex\memory.sqlite3` on Windows, or `$HOME/.exocortex/memory.sqlite3` elsewhere. For an isolated development DB use `cargo run -p exo-cli -- --db .\test-memory.sqlite3`.

Run `:help` for all commands, including `:remember-kind`, `:remember-topic`, `:conflicts`, `:correct`, `:forget`, `:export` and `:backup`.

**Privacy:** The local database and its backups/exports can contain personal information. Keep them out of Git, avoid storing secrets in V0.2, and read [memory semantics and limits](docs/MEMORY_V02.md) before using deletion or backup features.

## Structure

```text
crates/cognitive-core/src/
  event.rs         typed events
  memory.rs        typed memory and provenance
  permission.rs    permission boundary (V0.1)
  kernel.rs        in-memory session kernel (V0.1)
  store.rs         SQLite-backed V0.2 memory
crates/cognitive-core/tests/persistence.rs
apps/exo-cli/src/main.rs
```

V0.3 will add evidence-linked working-state updates, source freshness and deterministic checks before tool execution. JARVIS remains a separate project until a stable, permission-aware cognitive API is available.
