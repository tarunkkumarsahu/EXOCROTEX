#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use cognitive_core::{
    CognitiveEvent, EpistemicType, EventSource, FactKind, MemoryConflict, MemoryKind, Observation,
    PersistentMemoryStore, StoredMemory, WorkingFact,
};
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

#[derive(Clone)]
struct DatabasePath(PathBuf);

type CommandResult<T> = Result<T, String>;

fn database_path() -> CommandResult<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or("Missing USERPROFILE/HOME; cannot locate existing database")?;
    Ok(PathBuf::from(home).join(".exocortex").join("memory.sqlite3"))
}

fn with_store<T>(state: &DatabasePath, op: impl FnOnce(&mut PersistentMemoryStore) -> CommandResult<T>) -> CommandResult<T> {
    // Separate SQLite connection per command; the existing V0.2/V0.3 schema and DB are reused.
    let mut store = PersistentMemoryStore::open(&state.0).map_err(|err| err.to_string())?;
    op(&mut store)
}

#[derive(Serialize)]
struct WorkspaceStatus {
    memories: i64,
    events: i64,
    observations: i64,
    facts: i64,
    ai_connected: bool,
    jarvis_connected: bool,
}

#[tauri::command]
fn workspace_status(state: State<'_, DatabasePath>) -> CommandResult<WorkspaceStatus> {
    with_store(&state, |store| {
        Ok(WorkspaceStatus {
            memories: store.memory_count().map_err(|e| e.to_string())?,
            events: store.event_count().map_err(|e| e.to_string())?,
            observations: store.observation_count().map_err(|e| e.to_string())?,
            facts: store.working_fact_count().map_err(|e| e.to_string())?,
            ai_connected: false,
            jarvis_connected: false,
        })
    })
}

#[tauri::command]
fn list_memories(state: State<'_, DatabasePath>, query: String) -> CommandResult<Vec<StoredMemory>> {
    if query.len() > 250 { return Err("Search must be at most 250 characters".into()); }
    with_store(&state, |store| store.recall(&query).map_err(|e| e.to_string()))
}

#[tauri::command]
fn remember(state: State<'_, DatabasePath>, text: String, kind: MemoryKind, topic: Option<String>) -> CommandResult<StoredMemory> {
    if text.len() > 8000 || topic.as_ref().is_some_and(|t| t.len() > 120) {
        return Err("Memory text or topic too long".into());
    }
    with_store(&state, |store| store.create_memory(&text, kind, EpistemicType::UserConfirmedFact, topic.as_deref()).map_err(|e| e.to_string()))
}

#[tauri::command]
fn correct_memory(state: State<'_, DatabasePath>, id: Uuid, text: String) -> CommandResult<StoredMemory> {
    if text.len() > 8000 { return Err("Correction too long".into()); }
    with_store(&state, |store| store.correct(id, &text).map_err(|e| e.to_string()))
}

#[derive(Serialize)]
struct Redaction { redacted_revisions: usize }

#[tauri::command]
fn forget_memory(state: State<'_, DatabasePath>, id: Uuid) -> CommandResult<Redaction> {
    with_store(&state, |store| store.forget(id).map(|redacted_revisions| Redaction { redacted_revisions }).map_err(|e| e.to_string()))
}

#[tauri::command]
fn memory_source(state: State<'_, DatabasePath>, id: Uuid) -> CommandResult<CognitiveEvent> {
    with_store(&state, |store| store.source(id).map_err(|e| e.to_string())?.ok_or("Source event not found".into()))
}

#[tauri::command]
fn memory_conflicts(state: State<'_, DatabasePath>, id: Uuid) -> CommandResult<Vec<MemoryConflict>> {
    with_store(&state, |store| store.possible_conflicts(id).map_err(|e| e.to_string()))
}

#[tauri::command]
fn list_facts(state: State<'_, DatabasePath>, entity: String) -> CommandResult<Vec<WorkingFact>> {
    if entity.len() > 120 { return Err("Entity too long".into()); }
    with_store(&state, |store| {
        (if entity.trim().is_empty() { store.all_working_facts() } else { store.working_facts(&entity) })
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn record_observation(state: State<'_, DatabasePath>, source_key: String, version: i64, value: String) -> CommandResult<Observation> {
    if source_key.len() > 120 || value.len() > 8000 { return Err("Observation fields too long".into()); }
    with_store(&state, |store| store.record_observation(&source_key, version, &value, EventSource::User).map_err(|e| e.to_string()))
}

#[tauri::command]
fn latest_observation(state: State<'_, DatabasePath>, source_key: String) -> CommandResult<Observation> {
    with_store(&state, |store| store.latest_observation(&source_key).map_err(|e| e.to_string())?.ok_or("No observation for this source".into()))
}

#[tauri::command]
fn add_working_fact(state: State<'_, DatabasePath>, entity: String, attribute: String, value: String, kind: FactKind, evidence_ids: Vec<Uuid>) -> CommandResult<WorkingFact> {
    if entity.len() > 120 || attribute.len() > 120 || value.len() > 8000 || evidence_ids.len() > 20 {
        return Err("Fact fields or evidence list too long".into());
    }
    with_store(&state, |store| store.create_working_fact(&entity, &attribute, &value, kind, &evidence_ids).map_err(|e| e.to_string()))
}

fn main() {
    let db_path = database_path().expect("Unable to determine EXOCORTEX database location");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).expect("Unable to prepare EXOCORTEX database directory");
    }
    PersistentMemoryStore::open(&db_path).expect("Unable to open existing EXOCORTEX database");
    tauri::Builder::default()
        .manage(DatabasePath(db_path))
        .invoke_handler(tauri::generate_handler![
            workspace_status,
            list_memories,
            remember,
            correct_memory,
            forget_memory,
            memory_source,
            memory_conflicts,
            list_facts,
            record_observation,
            latest_observation,
            add_working_fact,
        ])
        .run(tauri::generate_context!())
        .expect("EXOCORTEX desktop application failed to start");
}
