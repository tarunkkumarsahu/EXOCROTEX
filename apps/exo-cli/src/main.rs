use std::io::{self, Write};
use std::path::PathBuf;

use cognitive_core::{
    ActionRisk, ApprovalState, CognitiveKernel, EpistemicType, EventSource, FactKind, MemoryKind,
    PersistentMemoryStore, StoreError,
};
use serde_json::json;
use uuid::Uuid;

fn main() {
    if let Err(error) = run() {
        eprintln!("EXOCORTEX could not start: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), StoreError> {
    let database_path = database_path()?;
    if let Some(parent) = database_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut store = PersistentMemoryStore::open(&database_path)?;
    let mut kernel = CognitiveKernel::new();

    println!("EXOCORTEX Cognitive Kernel v0.3");
    println!("Persistent memory database: {}", database_path.display());
    println!("Autonomous cognition, permissioned action.");
    println!("Type :help for commands.\n");

    loop {
        print!("exo> ");
        if io::stdout().flush().is_err() {
            break;
        }
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("input error: {error}");
                break;
            }
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if matches!(input, ":exit" | ":quit") {
            break;
        }
        if input == ":help" {
            print_help();
            continue;
        }
        if input == ":status" {
            println!(
                "persistent_events={} active_memories={} observations={} working_facts={} pending_approvals={} (approvals are session-only)",
                store.event_count()?, store.memory_count()?, store.observation_count()?, store.working_fact_count()?, kernel.pending_actions().len()
            );
            continue;
        }
        if let Some(text) = input.strip_prefix(":remember ") {
            report_save(&mut store, text, MemoryKind::Semantic, None);
            continue;
        }
        if let Some(args) = input.strip_prefix(":remember-kind ") {
            match args.split_once(' ') {
                Some((kind, text)) => match parse_kind(kind) {
                    Some(kind) => report_save(&mut store, text, kind, None),
                    None => {
                        println!("unknown kind: {kind}; use working|episodic|semantic|prospective")
                    }
                },
                None => println!("usage: :remember-kind <kind> <text>"),
            }
            continue;
        }
        if let Some(args) = input.strip_prefix(":remember-topic ") {
            match args.split_once(' ') {
                Some((topic, text)) => {
                    report_save(&mut store, text, MemoryKind::Semantic, Some(topic))
                }
                None => println!("usage: :remember-topic <topic-key> <text>"),
            }
            continue;
        }
        if let Some(args) = input.strip_prefix(":observe ") {
            let mut parts = args.splitn(3, ' ');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(source), Some(version), Some(value)) => match version.parse::<i64>() {
                    Ok(version) => {
                        match store.record_observation(source, version, value, EventSource::User) {
                            Ok(obs) => println!(
                                "observation={} source={} version={} event={}",
                                obs.id, obs.source_key, obs.version, obs.event_id
                            ),
                            Err(error) => println!("observation failed: {error}"),
                        }
                    }
                    Err(_) => println!("version must be a positive integer"),
                },
                _ => println!("usage: :observe <source-key> <version> <reported value>"),
            }
            continue;
        }
        if let Some(source) = input.strip_prefix(":latest ") {
            match store.latest_observation(source) {
                Ok(Some(obs)) => println!(
                    "{} | {} | v{} | {} | event={} | at={}",
                    obs.id,
                    obs.source_key,
                    obs.version,
                    obs.value,
                    obs.event_id,
                    obs.observed_at.to_rfc3339()
                ),
                Ok(None) => println!("no observations for that source"),
                Err(error) => println!("lookup failed: {error}"),
            }
            continue;
        }
        if let Some(raw_id) = input.strip_prefix(":observation ") {
            match parse_uuid(raw_id).and_then(|id| store.observation(id).map_err(|e| e.to_string()))
            {
                Ok(Some(obs)) => println!(
                    "{} | {} | v{} | {} | event={} | at={}",
                    obs.id,
                    obs.source_key,
                    obs.version,
                    obs.value,
                    obs.event_id,
                    obs.observed_at.to_rfc3339()
                ),
                Ok(None) => println!("observation not found"),
                Err(error) => println!("lookup failed: {error}"),
            }
            continue;
        }
        if let Some(args) = input.strip_prefix(":fact ") {
            // :fact observed project status building | <observation-uuid>[,<observation-uuid>]
            match args.split_once(" | ") {
                Some((claim, raw_ids)) => {
                    let mut fields = claim.splitn(4, ' ');
                    match (fields.next(), fields.next(), fields.next(), fields.next()) {
                        (Some(kind), Some(entity), Some(attribute), Some(value)) => {
                            let kind = match kind.to_lowercase().as_str() {
                                "observed" => Some(FactKind::Observed),
                                "derived" => Some(FactKind::Derived),
                                _ => None,
                            };
                            if let Some(kind) = kind {
                                let ids: Result<Vec<_>, String> = raw_ids.split(',').map(parse_uuid).collect();
                                match ids.and_then(|ids| store.create_working_fact(
                                    entity, attribute, value, kind, &ids
                                ).map_err(|e| e.to_string())) {
                                    Ok(fact) => println!("fact={} status={:?} evidence={:?}",
                                        fact.id, fact.status, fact.evidence_ids),
                                    Err(error) => println!("fact creation failed: {error}"),
                                }
                            } else {
                                println!("kind must be observed or derived");
                            }
                        },
                        _ => println!("usage: :fact <observed|derived> <entity> <attribute> <value> | <observation-uuid>[,<uuid>]"),
                    }
                },
                None => println!("usage: :fact <observed|derived> <entity> <attribute> <value> | <observation-uuid>[,<uuid>]"),
            }
            continue;
        }
        if let Some(entity) = input.strip_prefix(":facts ") {
            match store.working_facts(entity) {
                Ok(facts) if facts.is_empty() => println!("no working facts for this entity"),
                Ok(facts) => {
                    for fact in facts {
                        println!(
                            "{} | {}.{}={} | {:?} | evidence={:?} | stale_at={}",
                            fact.id,
                            fact.entity,
                            fact.attribute,
                            fact.value,
                            fact.status,
                            fact.evidence_ids,
                            fact.stale_at
                                .map(|date| date.to_rfc3339())
                                .unwrap_or_else(|| "-".to_string())
                        );
                    }
                }
                Err(error) => println!("working-state lookup failed: {error}"),
            }
            continue;
        }
        if input == ":recall" || input.starts_with(":recall ") {
            let query = input.strip_prefix(":recall").unwrap_or("").trim();
            match store.recall(query) {
                Ok(memories) if memories.is_empty() => println!("no matching active memories"),
                Ok(memories) => {
                    for stored in memories {
                        let memory = &stored.memory;
                        println!(
                            "{} | {:?} | {:?} | {} | source={} | created={} | topic={}",
                            memory.id,
                            memory.kind,
                            memory.epistemic_type,
                            memory.text,
                            memory.source_event_id,
                            memory.created_at.to_rfc3339(),
                            stored.topic_key.as_deref().unwrap_or("-")
                        );
                    }
                }
                Err(error) => println!("recall failed: {error}"),
            }
            continue;
        }
        if let Some(id) = input.strip_prefix(":source ") {
            match parse_uuid(id).and_then(|id| store.source(id).map_err(|e| e.to_string())) {
                Ok(Some(event)) => println!(
                    "{} | {:?} | {:?} | {} | {}",
                    event.id,
                    event.kind,
                    event.source,
                    event.timestamp.to_rfc3339(),
                    event.content
                ),
                Ok(None) => println!("source event not found"),
                Err(error) => println!("source failed: {error}"),
            }
            continue;
        }
        if let Some(id) = input.strip_prefix(":conflicts ") {
            match parse_uuid(id)
                .and_then(|id| store.possible_conflicts(id).map_err(|e| e.to_string()))
            {
                Ok(matches) if matches.is_empty() => {
                    println!("no differing active claims under this topic")
                }
                Ok(matches) => {
                    for conflict in matches {
                        println!(
                            "POTENTIAL CONFLICT | topic={} | other_id={} | {}",
                            conflict.topic_key, conflict.other_id, conflict.other_text
                        );
                    }
                }
                Err(error) => println!("conflict check failed: {error}"),
            }
            continue;
        }
        if let Some(args) = input.strip_prefix(":correct ") {
            match args.split_once(' ') {
                Some((id, text)) => match parse_uuid(id)
                    .and_then(|id| store.correct(id, text).map_err(|e| e.to_string()))
                {
                    Ok(stored) => println!(
                        "corrected; new current memory={} source={}",
                        stored.memory.id, stored.memory.source_event_id
                    ),
                    Err(error) => println!("correction failed: {error}"),
                },
                None => println!("usage: :correct <memory-uuid> <replacement text>"),
            }
            continue;
        }
        if let Some(raw_id) = input.strip_prefix(":forget ") {
            match parse_uuid(raw_id).and_then(|id| store.forget(id).map_err(|e| e.to_string())) {
                Ok(count) => println!("redacted {count} memory revision(s) and source text in local DB; existing backups are unchanged"),
                Err(error) => println!("forget failed: {error}"),
            }
            continue;
        }
        if let Some(path) = input.strip_prefix(":export ") {
            match store.export_json(path.trim()) {
                Ok(count) => println!("exported {count} current memories to {}", path.trim()),
                Err(error) => println!("export failed: {error}"),
            }
            continue;
        }
        if let Some(path) = input.strip_prefix(":backup ") {
            match store.backup(path.trim()) {
                Ok(()) => println!("SQLite backup saved to {}", path.trim()),
                Err(error) => println!("backup failed: {error}"),
            }
            continue;
        }
        if let Some(summary) = input.strip_prefix(":propose ") {
            let proposal = kernel.propose_action(
                "manual_external_action",
                summary,
                ActionRisk::ExternalWrite,
                json!({"summary": summary}),
            );
            println!(
                "proposal {} | risk={:?} | approval={:?}",
                proposal.id, proposal.risk, proposal.approval_state
            );
            if proposal.approval_state == ApprovalState::Pending {
                println!("human approval required before execution (no tool execution in V0.2)");
            }
            continue;
        }
        if input == ":pending" {
            let pending = kernel.pending_actions();
            if pending.is_empty() {
                println!("no pending approvals");
            } else {
                for action in pending {
                    println!("{} | {:?} | {}", action.id, action.risk, action.summary);
                }
            }
            continue;
        }
        if let Some(raw_id) = input.strip_prefix(":approve ") {
            handle_approval(&mut kernel, raw_id, true);
            continue;
        }
        if let Some(raw_id) = input.strip_prefix(":reject ") {
            handle_approval(&mut kernel, raw_id, false);
            continue;
        }
        if input.starts_with(':') {
            println!("unknown command; type :help");
            continue;
        }
        let event = kernel.observe_user(input);
        println!(
            "captured session event {}. Use :remember <text> to persist it.",
            event.id
        );
    }
    Ok(())
}

fn database_path() -> Result<PathBuf, StoreError> {
    let mut args = std::env::args_os().skip(1);
    match args.next() {
        Some(flag) if flag.to_string_lossy() == "--db" => args
            .next()
            .map(PathBuf::from)
            .ok_or(StoreError::InvalidInput("--db requires a path")),
        Some(_) => Err(StoreError::InvalidInput("usage: exo-cli [--db PATH]")),
        None => {
            let home = std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .ok_or(StoreError::InvalidInput(
                    "USERPROFILE/HOME not set; use --db PATH",
                ))?;
            Ok(PathBuf::from(home)
                .join(".exocortex")
                .join("memory.sqlite3"))
        }
    }
}

fn parse_kind(input: &str) -> Option<MemoryKind> {
    match input.to_lowercase().as_str() {
        "working" => Some(MemoryKind::Working),
        "episodic" => Some(MemoryKind::Episodic),
        "semantic" => Some(MemoryKind::Semantic),
        "prospective" => Some(MemoryKind::Prospective),
        _ => None,
    }
}

fn report_save(
    store: &mut PersistentMemoryStore,
    text: &str,
    kind: MemoryKind,
    topic: Option<&str>,
) {
    match store.create_memory(text, kind, EpistemicType::UserConfirmedFact, topic) {
        Ok(stored) => {
            println!(
                "saved memory={} kind={:?} source={} at={}",
                stored.memory.id,
                stored.memory.kind,
                stored.memory.source_event_id,
                stored.memory.created_at.to_rfc3339()
            );
            match store.possible_conflicts(stored.memory.id) {
                Ok(conflicts) if !conflicts.is_empty() => {
                    println!("POTENTIAL CONFLICT: {} differing active claim(s) for this topic; use :conflicts <uuid>", conflicts.len());
                }
                Err(error) => println!("conflict check unavailable: {error}"),
                _ => {}
            }
        }
        Err(error) => println!("save failed: {error}"),
    }
}

fn parse_uuid(raw: &str) -> Result<Uuid, String> {
    Uuid::parse_str(raw.trim()).map_err(|error| format!("invalid UUID: {error}"))
}

fn handle_approval(kernel: &mut CognitiveKernel, raw_id: &str, approve: bool) {
    let id = match parse_uuid(raw_id) {
        Ok(id) => id,
        Err(error) => {
            println!("{error}");
            return;
        }
    };
    let result = if approve {
        kernel.approve_action(id)
    } else {
        kernel.reject_action(id)
    };
    match result {
        Some(action) => println!("{} -> {:?}", action.id, action.approval_state),
        None => println!("proposal not found"),
    }
}

fn print_help() {
    println!(
        r#"Evidence-linked working state (V0.3):
  :observe <source-key> <version> <value>  record source snapshot; newer versions stale old dependent facts
  :latest <source-key>                  show the latest observation for a source
  :observation <uuid>                   show a specific observation and source event ID
  :fact <observed|derived> <entity> <attribute> <value> | <observation-uuid>[,<uuid>]
                                         create evidence-linked working fact
  :facts <entity>                       inspect observed, derived, disputed and stale facts
Persistent-memory commands:
  :remember <text>                    user-confirmed semantic memory
  :remember-kind <kind> <text>        working|episodic|semantic|prospective
  :remember-topic <key> <text>       save under explicit topic, checking differing claims
  :recall [query]                    list current memories with source ID and timestamp
  :source <source-uuid>              inspect original source event
  :conflicts <memory-uuid>           view potential disagreements under same topic
  :correct <memory-uuid> <new text>  create a new revision, supersede old memory
  :forget <memory-uuid>              redact a memory and its correction chain in local DB
  :export <new-file.json>            export current memories + source events
  :backup <new-file.sqlite3>         standalone SQLite backup including revision history
Other commands:
  :propose <summary>                 create session-only external action proposal
  :pending | :approve <uuid> | :reject <uuid>
  :status | :help | :exit
Launch with --db PATH to choose a database. Default: ~/.exocortex/memory.sqlite3.
No AI model or actual external tool execution in V0.3. :observe is a MANUAL report, not an independently verified tool reading.
"#
    );
}
