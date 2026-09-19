# EXOCORTEX

Rust-first experimental human cognitive extension. The human retains authority over consequential actions. Model and tool integrations are optional, replaceable layers around an evidence-linked cognitive kernel.

## Current build

- **V0.1:** Rust cognitive events, memory/provenance types, human-approval prototype and CLI.
- **V0.2:** Persistent SQLite memory with revisions, redaction, topic flags, and restart tests.
- **V0.3:** Versioned observations, evidence-linked working facts, stale/disputed statuses.
- **V0.4:** Loopback Rust HTTP API for memory and evidence, with API integration tests.
- **V0.5:** React + TypeScript visual workspace for real API-backed memory, evidence and status.
- **V0.6:** Tauri 2 Windows desktop shell with direct Rust cognitive commands and the same SQLite memory database.
- **V0.7:** Read-only local AI chat through Ollama, bounded lexical retrieval of saved memories and evidence, inspectable context IDs, and a native chat page.

**Not implemented:** JARVIS integration, autonomous external-action execution, a verified multi-step reasoning loop or independently verified AI answers. A local model can answer questions, but a fluent answer is not proof. Installer signing and distribution remain separate work.

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

## Local AI chat (V0.7)

Install Ollama locally, then in PowerShell run `ollama pull qwen2.5:3b`. Launch the desktop app with `cd apps\exo-web` and `npm run desktop`, open **Cognitive chat**, press **Refresh**, and choose an installed model. Each question uses a small selected set of existing SQLite memories and facts; expand **Context supplied** to inspect IDs and statuses. Model output is AI-generated and can be mistaken. No chat transcript is automatically saved and no external actions or JARVIS tools are executed. If Ollama is not running, the app shows an explicit offline state rather than a fabricated response.

See [V0.7 local-model setup and privacy boundaries](docs/LOCAL_AI_V07.md) for the actual data flow and limitations.
