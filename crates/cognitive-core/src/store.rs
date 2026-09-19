//! Local V0.2 memory store. Events and memory updates are committed in one SQLite transaction.
//! An explicit topic key enables *potential* conflict detection, not semantic truth verification.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CognitiveEvent, EpistemicType, EventKind, EventSource, MemoryKind, MemoryRecord};

#[derive(Debug)]
pub enum StoreError {
    Database(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
    InvalidInput(&'static str),
    NotFound,
    InactiveMemory,
    PathExists,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(err) => write!(f, "SQLite: {err}"),
            Self::Json(err) => write!(f, "JSON: {err}"),
            Self::Io(err) => write!(f, "I/O: {err}"),
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::NotFound => write!(f, "memory/event not found"),
            Self::InactiveMemory => write!(f, "memory was deleted or superseded"),
            Self::PathExists => write!(f, "destination already exists; choose a new path"),
        }
    }
}

impl std::error::Error for StoreError {}
impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMemory {
    pub memory: MemoryRecord,
    pub topic_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConflict {
    pub memory_id: Uuid,
    pub other_id: Uuid,
    pub topic_key: String,
    pub other_text: String,
}

#[derive(Debug, Serialize)]
struct MemoryExport {
    format: &'static str,
    exported_at: DateTime<Utc>,
    memories: Vec<ExportEntry>,
}

#[derive(Debug, Serialize)]
struct ExportEntry {
    record: StoredMemory,
    source: CognitiveEvent,
}

pub struct PersistentMemoryStore {
    conn: Connection,
}

impl PersistentMemoryStore {
    /// Open a local SQLite DB; parent directory must exist. Use the same path across sessions.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             PRAGMA secure_delete=ON;
             PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                event_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                record_json TEXT NOT NULL,
                source_event_id TEXT NOT NULL,
                topic_key TEXT,
                deleted_at TEXT,
                FOREIGN KEY(source_event_id) REFERENCES events(id)
             );
             CREATE INDEX IF NOT EXISTS idx_memories_topic ON memories(topic_key);",
        )?;
        Ok(Self { conn })
    }

    pub fn create_memory(
        &mut self,
        text: &str,
        kind: MemoryKind,
        epistemic_type: EpistemicType,
        topic: Option<&str>,
    ) -> Result<StoredMemory> {
        if text.trim().is_empty() {
            return Err(StoreError::InvalidInput("memory text must not be empty"));
        }
        let topic_key = topic.map(normalize_topic).transpose()?;
        let event = CognitiveEvent::new(
            EventKind::UserStatement,
            EventSource::User,
            text.trim(),
            1.0,
        );
        let memory = MemoryRecord::new(text.trim(), kind, epistemic_type, event.id);
        let stored = StoredMemory { memory, topic_key };
        let tx = self.conn.transaction()?;
        Self::insert_event(&tx, &event)?;
        Self::insert_memory(&tx, &stored)?;
        tx.commit()?;
        Ok(stored)
    }

    /// Includes only current, non-deleted memories; superseded revisions stay available
    /// through source/history inspection but never surface in ordinary recall.
    pub fn recall(&self, query: &str) -> Result<Vec<StoredMemory>> {
        let query = query.trim().to_lowercase();
        let mut stmt = self.conn.prepare(
            "SELECT record_json, topic_key FROM memories WHERE deleted_at IS NULL ORDER BY rowid DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })?;
        let mut found = Vec::new();
        for row in rows {
            let (data, topic_key) = row?;
            let memory: MemoryRecord = serde_json::from_str(&data)?;
            if memory.superseded_by.is_none()
                && (query.is_empty()
                    || memory.text.to_lowercase().contains(&query)
                    || topic_key
                        .as_ref()
                        .is_some_and(|topic| topic.contains(&query)))
            {
                found.push(StoredMemory { memory, topic_key });
            }
        }
        Ok(found)
    }

    pub fn active_memory(&self, id: Uuid) -> Result<StoredMemory> {
        let stored = self.load_memory(id)?.ok_or(StoreError::NotFound)?;
        let deleted: Option<String> = self.conn.query_row(
            "SELECT deleted_at FROM memories WHERE id=?1",
            params![id.to_string()],
            |r| r.get(0),
        )?;
        if deleted.is_some() || stored.memory.superseded_by.is_some() {
            return Err(StoreError::InactiveMemory);
        }
        Ok(stored)
    }

    pub fn source(&self, id: Uuid) -> Result<Option<CognitiveEvent>> {
        let payload: Option<String> = self
            .conn
            .query_row(
                "SELECT event_json FROM events WHERE id=?1",
                params![id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        payload
            .map(|value| serde_json::from_str(&value).map_err(StoreError::from))
            .transpose()
    }

    /// Corrections create a new memory and source event rather than overwriting history.
    pub fn correct(&mut self, id: Uuid, replacement_text: &str) -> Result<StoredMemory> {
        if replacement_text.trim().is_empty() {
            return Err(StoreError::InvalidInput("replacement must not be empty"));
        }
        let old = self.active_memory(id)?;
        let event = CognitiveEvent::new(
            EventKind::UserStatement,
            EventSource::User,
            replacement_text.trim(),
            1.0,
        );
        let mut new_memory = MemoryRecord::new(
            replacement_text.trim(),
            old.memory.kind.clone(),
            EpistemicType::UserConfirmedFact,
            event.id,
        );
        let now = Utc::now();
        new_memory.valid_from = now;
        let updated = StoredMemory {
            memory: new_memory,
            topic_key: old.topic_key.clone(),
        };
        let mut superseded = old.memory;
        superseded.superseded_by = Some(updated.memory.id);
        superseded.valid_until = Some(now);
        let audit = CognitiveEvent::new(
            EventKind::MemorySuperseded,
            EventSource::User,
            format!("{} -> {}", id, updated.memory.id),
            1.0,
        );
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE memories SET record_json=?1 WHERE id=?2 AND deleted_at IS NULL",
            params![serde_json::to_string(&superseded)?, id.to_string()],
        )?;
        if changed != 1 {
            return Err(StoreError::InactiveMemory);
        }
        Self::insert_event(&tx, &event)?;
        Self::insert_memory(&tx, &updated)?;
        Self::insert_event(&tx, &audit)?;
        tx.commit()?;
        Ok(updated)
    }

    /// Potential conflicts are limited to differing *current* claims with the SAME explicit
    /// topic key; this is not a semantic contradiction classifier or a truth judgment.
    pub fn possible_conflicts(&self, id: Uuid) -> Result<Vec<MemoryConflict>> {
        let current = self.active_memory(id)?;
        let Some(topic) = current.topic_key else {
            return Ok(Vec::new());
        };
        let mut conflicts = Vec::new();
        for candidate in self.recall("")? {
            if candidate.memory.id != id
                && candidate.topic_key.as_deref() == Some(topic.as_str())
                && normalize_text(&candidate.memory.text) != normalize_text(&current.memory.text)
            {
                conflicts.push(MemoryConflict {
                    memory_id: id,
                    other_id: candidate.memory.id,
                    topic_key: topic.clone(),
                    other_text: candidate.memory.text,
                });
            }
        }
        Ok(conflicts)
    }

    /// Privacy-oriented delete: tombstones every revision in the correction chain and
    /// redacts its original source events. Does not revoke existing external backups or
    /// guarantee forensic erasure from disks/OS snapshots.
    pub fn forget(&mut self, id: Uuid) -> Result<usize> {
        if self.load_memory(id)?.is_none() {
            return Err(StoreError::NotFound);
        }
        let mut all = self.all_memories()?;
        let mut chain: HashSet<Uuid> = HashSet::from([id]);
        loop {
            let size = chain.len();
            for (memory, _) in &all {
                if chain.contains(&memory.id)
                    || memory
                        .superseded_by
                        .is_some_and(|successor| chain.contains(&successor))
                {
                    chain.insert(memory.id);
                    if let Some(successor) = memory.superseded_by {
                        chain.insert(successor);
                    }
                }
            }
            if size == chain.len() {
                break;
            }
        }
        all.retain(|(memory, _)| chain.contains(&memory.id));
        let tx = self.conn.transaction()?;
        for (memory, _) in &all {
            let mut memory = memory.clone();
            memory.text = "[REDACTED BY USER]".to_string();
            memory.provenance.clear();
            tx.execute(
                "UPDATE memories SET record_json=?1, deleted_at=?2, topic_key=NULL WHERE id=?3",
                params![
                    serde_json::to_string(&memory)?,
                    Utc::now().to_rfc3339(),
                    memory.id.to_string()
                ],
            )?;
            let payload: String = tx.query_row(
                "SELECT event_json FROM events WHERE id=?1",
                params![memory.source_event_id.to_string()],
                |row| row.get(0),
            )?;
            let mut source: CognitiveEvent = serde_json::from_str(&payload)?;
            // Source events may have been shared by several memories. A privacy deletion
            // favors redacting that shared source over retaining potentially sensitive text.
            source.content = "[REDACTED BY USER]".to_string();
            tx.execute(
                "UPDATE events SET event_json=?1 WHERE id=?2",
                params![serde_json::to_string(&source)?, source.id.to_string()],
            )?;
        }
        let audit = CognitiveEvent::new(
            EventKind::MemoryDeleted,
            EventSource::User,
            format!("redacted {} memory revision(s)", all.len()),
            1.0,
        );
        Self::insert_event(&tx, &audit)?;
        tx.commit()?;
        // SQLite secure_delete applies to modified pages. Checkpoint shrinks the local WAL;
        // copies, physical flash remnants, and old backups remain outside this guarantee.
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(all.len())
    }

    /// JSON export intentionally contains only currently active memories and source events.
    pub fn export_json(&self, destination: impl AsRef<Path>) -> Result<usize> {
        let path = destination.as_ref();
        if path.exists() {
            return Err(StoreError::PathExists);
        }
        let mut entries = Vec::new();
        for record in self.recall("")? {
            let source = self
                .source(record.memory.source_event_id)?
                .ok_or(StoreError::NotFound)?;
            entries.push(ExportEntry { record, source });
        }
        let export = MemoryExport {
            format: "exocortex-memory-v0.2",
            exported_at: Utc::now(),
            memories: entries,
        };
        let json = serde_json::to_vec_pretty(&export)?;
        // create_new avoids unexpectedly overwriting a user's existing file.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        use std::io::Write;
        file.write_all(&json)?;
        file.sync_all()?;
        Ok(export.memories.len())
    }

    /// SQLite's VACUUM INTO produces a consistent, standalone DB snapshot.
    /// Choose a new file name; this backup includes audit/revision history.
    pub fn backup(&self, destination: impl AsRef<Path>) -> Result<()> {
        let path = destination.as_ref();
        if path.exists() {
            return Err(StoreError::PathExists);
        }
        let dest = path
            .to_str()
            .ok_or(StoreError::InvalidInput("backup path must be UTF-8"))?;
        self.conn.execute("VACUUM INTO ?1", params![dest])?;
        Ok(())
    }

    pub fn event_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?)
    }
    pub fn memory_count(&self) -> Result<i64> {
        Ok(self.recall("")?.len() as i64)
    }

    fn insert_event(tx: &Transaction<'_>, event: &CognitiveEvent) -> Result<()> {
        tx.execute(
            "INSERT INTO events(id, event_json) VALUES (?1, ?2)",
            params![event.id.to_string(), serde_json::to_string(event)?],
        )?;
        Ok(())
    }
    fn insert_memory(tx: &Transaction<'_>, stored: &StoredMemory) -> Result<()> {
        tx.execute(
            "INSERT INTO memories(id, record_json, source_event_id, topic_key, deleted_at)
             VALUES (?1, ?2, ?3, ?4, NULL)",
            params![
                stored.memory.id.to_string(),
                serde_json::to_string(&stored.memory)?,
                stored.memory.source_event_id.to_string(),
                &stored.topic_key
            ],
        )?;
        Ok(())
    }
    fn load_memory(&self, id: Uuid) -> Result<Option<StoredMemory>> {
        let row: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT record_json, topic_key FROM memories WHERE id=?1",
                params![id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(data, topic_key)| {
            Ok(StoredMemory {
                memory: serde_json::from_str(&data)?,
                topic_key,
            })
        })
        .transpose()
    }
    fn all_memories(&self) -> Result<Vec<(MemoryRecord, Option<String>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT record_json, topic_key FROM memories")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?;
        let mut all = Vec::new();
        for row in rows {
            let (data, key) = row?;
            all.push((serde_json::from_str(&data)?, key));
        }
        Ok(all)
    }
}

fn normalize_topic(topic: &str) -> Result<String> {
    let normalized = topic.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(StoreError::InvalidInput("topic key must not be empty"));
    }
    Ok(normalized)
}
fn normalize_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
