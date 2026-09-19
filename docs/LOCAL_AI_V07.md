# V0.7 — Local AI cognitive chat

V0.7 adds an **experimental, read-only local LLM adapter** to the existing V0.6 Tauri desktop app. It calls an installed Ollama model on \`http://127.0.0.1:11434\` directly from Rust, never from the browser. This is a real model request when Ollama is running and a model is installed — not a mock response. The browser dashboard remains usable for memory and evidence but displays a desktop-only notice for AI chat.

## Setup on Windows

1. Install and start Ollama from the official Windows installer: https://ollama.com/download/windows
2. In PowerShell, install a local model (example below is roughly 1.9 GB, plus inference memory requirements):

   \`\`\`powershell
   ollama pull qwen2.5:3b
   ollama list
   \`\`\`

3. From the **inner Git repository root** (the directory containing \`Cargo.toml\` and \`apps\`):

   \`\`\`powershell
   cd .\apps\exo-web
   npm install
   npm run desktop
   \`\`\`

4. Open **Cognitive chat**, click **Refresh** and choose an installed local model. Ask: \`What do my saved memories say about EXOCORTEX?\` Use the "Context supplied" button to inspect the actual stored records passed to the model.
5. To test persistence, save a new project memory in Memory vault, close and reopen the app, then ask a question containing its key terms. The **saved memory persists**; the **chat transcript is session-only**.

If Ollama is not started, you will see an explicit offline message. If no model is installed, the model list remains empty. The app does not silently substitute a canned reply or claim that JARVIS is connected.

## Design and trust boundaries

- Rust opens the existing \`%USERPROFILE%\.exocortex\memory.sqlite3\` read-only *for AI chat operations* in the sense that the chat workflow performs no database writes. The separate Memory Vault remains responsible for explicit memory edits.
- For each question, it selects at most 6 relevant active memories and 4 relevant working facts via **simple lexical overlap**; it does not yet use embeddings, semantic retrieval or robust multilingual search. Generic recall prompts include a small number of recent active memories. Irrelevant memory is not intentionally sent.
- The UI displays every selected record's UUID, source event UUID where applicable, type and stale/disputed status. The model is instructed to cite matching \`[M1]\`/\`[F1]\` labels, but citations and factual correctness **are not independently verified**. All user-recorded facts can be mistaken.
- Local messages (question + up to six trimmed recent turns + selected context) are sent only to \`127.0.0.1:11434\`. No cloud key or provider is configured. A locally installed Ollama model and its runtime may have their own logs/settings; this app does not control them.
- This feature does **not** automatically persist chat content as memory and does **not** execute external actions or use tools. The model never receives a tool registry. The installed model must be selected from the locally reported model list.
- A retrieved record may contain adversarial instructions. It is clearly marked untrusted and supplied as data, but **model-level prompt-injection resistance is not guaranteed**. Do not store secrets or rely on the model to enforce security boundaries.
- No JARVIS bridge, multi-step agent loop, cloud synchronization, autonomous decisions or experimental proof of cognitive enhancement are included in V0.7.

## Repeatable local checks

\`\`\`powershell
cargo test --workspace
cd .\apps\exo-web
npm run build
npm run desktop
\`\`\`

The \`src-tauri\` folder is a separate Rust package. On Windows, run \`npm run desktop\` or \`npm run desktop:build\` to compile and test its actual native integration. The \`ai.rs\` unit tests cover deterministic context retrieval, provenance and rejection of untrusted chat roles; they do not simulate a genuine model's reasoning quality.

## Next validation

Compare zero-memory questions, one relevant active memory, conflicting or stale evidence and an unavailable Ollama service. Record any citation errors rather than treating a fluent response as independently verified. Next development phases should improve semantic retrieval and source validation, then add action safety and JARVIS integration through a separate permission boundary.
