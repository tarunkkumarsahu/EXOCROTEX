# EXOCORTEX V0.6 — Windows desktop shell

This is the existing V0.5 React workspace embedded in a Tauri 2 native desktop window. The Tauri process invokes `cognitive-core` Rust commands **directly**, so desktop users do not start `exo-api`, `npm run dev`, or a localhost listener. The browser version still uses `exo-api` and Vite's `/api` proxy.

## Windows prerequisites

- Working Rust stable MSVC toolchain + Visual Studio C++ Build Tools (Desktop development with C++)
- Microsoft Edge WebView2 Runtime (usually preinstalled in modern Windows)
- Node.js + npm

Run PowerShell **from your actual repository root** (the folder containing `Cargo.toml` and `apps`):

```powershell
# First time only; you may have to install additional packages
cd .\apps\exo-web
npm install
npm run desktop
```

`npm run desktop` opens a native window; keep this terminal running during **development**. To build a standalone Windows installer:

```powershell
cd .\apps\exo-web
npm run desktop:build
```

When the build succeeds, the installer is under `apps\exo-web\src-tauri\target\release\bundle\nsis\`. Install it, then launch EXOCORTEX from the Start menu. No terminal or API server is needed for the **installed app**. The first build downloads dependencies and can take time. An installer build is not a signed public release; Windows SmartScreen may warn about unsigned development builds.

## Database and boundaries

The desktop shell opens the **same** `%USERPROFILE%\.exocortex\memory.sqlite3` database used by the V0.2/V0.3 CLI and V0.4 API. It does not overwrite existing records; create a private backup before upgrading or testing destructive UI operations. SQLite backups, exports and WAL files may contain personal data.

The desktop binary ships only the defined cognitive read/write commands. It does not start a broad localhost API server or install filesystem, shell, or browser permissions. The current `:forget` implementation only redacts memory and its source; it does **not** redact separate V0.3 observations and it does not revoke prior backups. Avoid storing secrets or highly sensitive data in this prototype.

**Not implemented here:** AI model reasoning, autonomous actions, scheduled tasks, JARVIS integration, or native file import. `Observed` and `Derived` statuses are still manual user reports, not proof of truth.

## Browser option retained

From repository root run `cargo run -p exo-api`. In another terminal run `cd apps\exo-web; npm run dev`; open `http://127.0.0.1:5173`. The desktop variant does **not** require these terminals.

## Verification

Windows build validation runs in GitHub Actions under `Desktop Windows`. The existing Rust CI only checks core/CLI/API, because native Tauri requires Windows/GTK-specific system dependencies. Do not treat an untested Tauri packaging config as a completed installer until the Windows build succeeds.
