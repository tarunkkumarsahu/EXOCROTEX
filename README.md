# EXOCORTEX

Rust-first experimental human cognitive extension. The human retains authority over consequential actions. Model and tool integrations are optional, replaceable layers around an evidence-linked cognitive kernel.

## Current build

- **V0.1:** Rust cognitive events, memory/provenance types, human-approval prototype and CLI.
- **V0.2:** Persistent SQLite memory with revisions, redaction, topic flags, and restart tests.
- **V0.3:** Versioned observations, evidence-linked working facts, stale/disputed statuses.
- **V0.4:** Loopback Rust HTTP API for memory and evidence, with API integration tests.
- **V0.5:** React + TypeScript visual workspace for real API-backed memory, evidence and status.
- **V0.6:** Tauri 2 Windows desktop shell with direct Rust cognitive commands and the same SQLite memory database.

**Not implemented:** AI model reasoning, JARVIS integration, autonomous external-action execution, or a signed public installer. The desktop development window works; CI packaging and release signing are separate checks.

## Run the visual workspace on Windows

Open two PowerShell terminals at the repository root:

```powershell
# terminal 1 — keep running
cargo run -p exo-api
```

```powershell
# terminal 2
cd apps\exo-web
npm install
npm run dev
```

Open `http://127.0.0.1:5173` in a browser. Existing memories are automatically loaded from `%USERPROFILE%\.exocortex\memory.sqlite3`, the same database used by V0.2/V0.3. **Do not delete or reset the database.**

The terminal CLI remains available with `cargo run -p exo-cli`, but it is no longer required for visual memory operations. For isolated API experiments: `cargo run -p exo-api -- --db .\demo-memory.sqlite3`.

More information: [workspace API, setup and security notes](docs/WORKSPACE_V04_V05.md), [V0.2 memory semantics](docs/MEMORY_V02.md), [V0.3 evidence semantics](docs/WORKING_STATE_V03.md).

## Verification

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps\exo-web
npm run build
```

The API is intentionally loopback-only; the simple local client header is not an authentication mechanism. Do not expose port 8787 publicly or put sensitive secrets in this experimental database.

## Windows desktop application (V0.6)

From the repository root, open PowerShell and run:

```powershell
cd apps\exo-web
npm install
npm run desktop
```

This starts the native **development** window. To build a standalone Windows installer instead, run `npm run desktop:build` in the same folder. The installed app opens the existing `%USERPROFILE%\.exocortex\memory.sqlite3` directly; unlike browser mode, it does not require a separate HTTP API process. See [V0.6 setup, data safeguards, and packaging status](docs/DESKTOP_V06.md). Keep personal SQLite data and backups out of Git.
