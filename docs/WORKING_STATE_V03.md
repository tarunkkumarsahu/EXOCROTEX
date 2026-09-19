# V0.3 — evidence-linked working state

This milestone adds a **manual, testable working-state layer** to the existing V0.2 SQLite database. It does not add an LLM, automatic inference, live GitHub/sensor integration, or real tool execution.

## Semantics

- **Observation:** an externally or manually reported snapshot identified by a `source_key`, positive strictly increasing integer `version`, event ID, observation ID, UTC timestamp and reported text. An observation only means that the indicated source reported a value; it does not independently establish truth.
- **Working fact:** a claim `(entity, attribute, value)` labeled *Observed* or *Derived*, with one or more supporting observation UUIDs. A derived claim is user-entered in this version: the program does not generate or verify reasoning.
- **Stale:** saving a newer observation of a source atomically marks every non-stale fact supported by an older version of that source as stale. Other unrelated facts remain unchanged. The old observation and fact stay inspectable.
- **Disputed:** different text values for the same explicit `(entity, attribute)` mark current facts as potentially disputed. This is a string comparison, not semantic contradiction detection or an independent truth decision. If a disputed fact becomes stale, the remaining current facts are recalculated.
- **Freshness:** comparisons use locally supplied monotonically increasing versions. Source versions must be assigned accurately by the caller; the program cannot detect an external change it has not been told about. A newer version invalidates old dependent facts even if its reported text is identical.

## PowerShell demo

Start with an isolated database (the default persistent memory DB also works):

```powershell
cargo run -p exo-cli -- --db .\working-demo.sqlite3
```

Inside `exo>`:

```text
:observe repo-main 1 commit-A
```

Copy the `observation=<uuid>` printed above and substitute it in the next command:

```text
:fact observed project commit A | <observation-uuid>
:facts project
:observe repo-main 2 commit-B
:facts project
:latest repo-main
:exit
```

The old fact's status should become `Stale`, with a `stale_at` time. Restart with the same `--db` path, run `:facts project`, and confirm the stale fact remains. To record a new current fact use the observation UUID for version 2 with `:fact observed project commit B | <new-observation-uuid>`.

You can cite multiple observations for a claim with comma-separated UUIDs. The CLI is deliberately explicit about evidence rather than allowing the user to create uncited current facts.

## Reliability and privacy notes

- `:observe` records a **manual user report**. It is not a GitHub read, sensor read or independently verified tool receipt.
- V0.3 does not automatically infer evidence dependencies from natural-language text. The caller chooses observation UUIDs; those dependencies determine invalidation.
- All V0.2 memories and their commands remain usable in the same database. SQLite schema changes use `CREATE TABLE IF NOT EXISTS` and do not overwrite old memory rows. Before major upgrades, create a fresh `:backup` and keep it private.
- Local SQLite/JSON backups and WAL files can contain personal data. Do not commit them or secret values to Git.
- Current V0.2 `:forget` redacts **memory** source events, not separate V0.3 working-state observations. Do not use V0.3 to store sensitive information until its own delete/export privacy controls exist.
- Approval proposals in the V0.1 kernel remain session-only. There is no actual external action execution or independent correctness guarantee.

## Next milestone

V0.4 can add a model-provider adapter that emits *proposed* structured observations/facts/actions through the Rust validation boundary. Evidence IDs, source versions, and authorization scope must be checked independently of a model's fluent explanation.
