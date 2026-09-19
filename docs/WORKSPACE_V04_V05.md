# V0.4–V0.5: local API and cognitive workspace

This release is a **working visual interface** for the previously implemented cognitive memory/evidence system. It does not connect an LLM, JARVIS, cloud account, or external tool.

## Windows quick start

Open two PowerShell terminals in the repository root. The project uses the same existing SQLite database by default (`%USERPROFILE%\.exocortex\memory.sqlite3`). You do **not** need to migrate or reset your V0.2/V0.3 memories.

Terminal 1 — Rust API:

```powershell
cargo run -p exo-api
```

Terminal 2 — React UI:

```powershell
cd apps\exo-web
npm install
npm run dev
```

Open `http://127.0.0.1:5173`. To stop, press Ctrl+C in both terminals. To return to the repo root from `apps\exo-web`, run `cd ..\..`.

If you want a separate test database instead of your real memories, start the API with `cargo run -p exo-api -- --db .\demo-memory.sqlite3` **before** launching the UI. Keep local DB files and exports out of Git.

## Working functionality

- API: `GET /api/health`, `GET /api/status`.
- Memories: list/search, create, correct (new revision), redact correction chain, inspect source event, compare differing active claims under an explicit topic.
- Evidence: manually record versioned source observations, inspect recent observations, create cited working facts, list all facts or filter by entity.
- UI: dashboard shows live database counts and records; memory vault supports source inspection, correction, conflict inspection, and user-confirmed deletion; evidence view supports manual observations and cited facts.

The UI uses Vite's development proxy so requests to `/api` reach the Rust API at `127.0.0.1:8787`. The API binds only to loopback. Requests require a non-simple `X-Exocortex-Client: local-ui-v1` header and permitted browser origins; this reduces ordinary cross-origin browser access, **but does not authenticate trusted local processes or protect against another application running as the same OS user**. Do not expose the port to the public internet or store secrets in the research prototype.

## Important semantic limitations

- Saving UI memory marks it as a **user-confirmed statement**, not an externally verified fact. The system does not currently infer or verify natural-language claims.
- Observations are **manual reports**, not live GitHub or sensor readings. Their version numbers are supplied by the human. Facts are stale only when the system is explicitly told about newer source versions.
- Database files are not encrypted by this prototype. Backups, exports, operating-system snapshots, and old copies are not automatically redacted if you delete a memory.
- V0.1 CLI action proposals are in-memory demonstrations and are **not** exposed as executable API actions. There is no email, shell, or OS-control endpoint.
- AI reasoning, Tauri Windows packaging, and the JARVIS bridge are separate upcoming milestones, and the interface labels them as **not connected**.

## API / UI boundary

The Rust API opens its own SQLite connection per short-lived blocking operation; no SQLite connection crosses async tasks. The existing cognitive-core functions implement memory and evidence semantics. React owns presentation only: it does not invent records or silently fabricate memory counts. Error messages and offline state are visible to the user.

## Next milestones

V0.6: package this React UI inside Tauri with an explicitly managed loopback API lifecycle and origin policy. V0.7: provider-neutral LLM adapter proposing typed cognition, validated by the kernel. V0.8: versioned JARVIS bridge with explicit user permissions and graceful standalone mode.
