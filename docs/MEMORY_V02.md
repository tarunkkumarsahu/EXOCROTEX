# V0.2 — Persistent cognitive memory

## Status / scope

A local SQLite memory store behind the existing Rust cognitive-core crate. V0.1's in-memory action proposals, approvals and free-form `observe_user` session events are **not yet persisted**. No LLM, verified external action execution, semantic embedding retrieval, encryption-at-rest, multi-user accounts, or JARVIS API exists in V0.2.

The default CLI database lives at `USERPROFILE/.exocortex/memory.sqlite3` on Windows and `HOME/.exocortex/memory.sqlite3` elsewhere. `--db PATH` overrides this for tests or isolated projects. On Windows the path is normally **outside the cloned repository and outside OneDrive**.

## Current memory semantics

- Four categorical kinds: working, episodic, semantic, prospective. Kind is a **label**; V0.2 does not automatically expire working memory or fire deadline reminders.
- Five epistemic labels: observation, user-confirmed fact, inference, hypothesis, prediction. CLI writes are user-confirmed; callers of the Rust API can set other labels. These labels are not guarantees of truth.
- Every stored memory has a UUID, creation and validity timestamps, an original source-event ID, and a provenance reference. The source event contains an independently recorded UUID, timestamp, kind, source and original text.
- `:correct OLD_ID NEW_TEXT` records another user statement, creates a new version and marks the old record as superseded. The old revision is omitted from normal recall; its source event remains available until forgotten.
- **Potential disagreement detection is deliberately narrow.** To compare two claims, store them using the same explicit topic key with `:remember-topic KEY TEXT`. Different current text values under that key are flagged for human review. The system **cannot identify semantic contradictions across arbitrary sentences**, decide which statement is true, or automatically supersede them.
- `:forget ID` redacts **all revisions in the correction chain**, redacts their source-event text, clears their topic keys, hides them from recall and active JSON export, and records a content-free deletion event. This changes original event payloads under a documented privacy-erasure exception; the event stream is otherwise append-only. A redacted event retains ID and timestamp for traceability.
- `:forget` is *not* guaranteed forensic erasure. Previously created backups/exports, other synced files, flash/OS snapshots and storage providers may still contain old material. `PRAGMA secure_delete=ON` and WAL checkpoint reduce, but do not eliminate, these risks. Don't use V0.2 for high-sensitivity secrets. Existing backups must be deleted separately.

## Commands

Run from project root:

```powershell
cargo test --workspace
cargo run -p exo-cli
# Or, for a throwaway / isolated database:
cargo run -p exo-cli -- --db .\sandbox-memory.sqlite3
```

Inside the CLI:

```text
:remember EXOCORTEX V0.2 tracks evidence-linked memories.
:remember-kind prospective Submit the experiment report by Friday.
:remember-topic project-milestone Our milestone is V0.2.
:remember-topic project-milestone Our milestone is V0.3.
:recall milestone
:conflicts <UUID_FROM_SECOND_TOPIC_MEMORY>
:source <SOURCE_UUID_FROM_RECALL>
:correct <MEMORY_UUID> Our milestone is V0.4.
:recall milestone
:export memory-export.json
:backup memory-backup.sqlite3
:forget <MEMORY_UUID>
:status
:exit
```

Restart using the **same `--db` path** or default database, then `:recall milestone`. The corrected current memory should remain, while superseded text should not appear in normal recall.

`:export` and `:backup` require filenames that **do not exist yet** and parent directories that already exist. Use a different output name for each run. The JSON export includes only active records and their sources. The SQLite backup contains revision/audit history and must be protected as private data. Do not commit SQLite databases or exported personal memories to GitHub.

## Acceptance tests

`crates/cognitive-core/tests/persistence.rs` includes integration tests for:

1. Restart/reopen persistence with original source and timestamp.
2. All four memory kinds and epistemic-label round-trip.
3. Correction, supersession and new provenance across reopen.
4. Potential conflicting claims under an explicit shared topic without auto-overwrite.
5. Redaction of a correction chain and its source text across reopen.
6. JSON export + standalone SQLite backup that can be opened independently.
7. Invalid input not creating orphaned event records.

The existing V0.1 permission boundary and its unit tests remain in place. A valid memory source is evidence of *who/what reported a statement*, not a certificate of objective truth.
