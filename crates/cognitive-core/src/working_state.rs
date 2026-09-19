//! Evidence-linked working state (V0.3). An observation is a reported source snapshot,
//! not a truth certificate. Revisions invalidate facts backed by older snapshots.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CognitiveEvent, EventKind, EventSource, PersistentMemoryStore, StoreError};

type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: Uuid,
    pub source_key: String,
    pub version: i64,
    pub value: String,
    pub event_id: Uuid,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactKind {
    Observed,
    Derived,
}

impl FactKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Derived => "derived",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "observed" => Ok(Self::Observed),
            "derived" => Ok(Self::Derived),
            _ => Err(StoreError::InvalidInput("unknown working-fact kind")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactStatus {
    Observed,
    Derived,
    Stale,
    Disputed,
}

impl FactStatus {
    fn from_str(value: &str) -> Result<Self> {
        match value {
            "observed" => Ok(Self::Observed),
            "derived" => Ok(Self::Derived),
            "stale" => Ok(Self::Stale),
            "disputed" => Ok(Self::Disputed),
            _ => Err(StoreError::InvalidInput("unknown working-fact status")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingFact {
    pub id: Uuid,
    pub entity: String,
    pub attribute: String,
    pub value: String,
    pub kind: FactKind,
    pub status: FactStatus,
    pub evidence_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub stale_at: Option<DateTime<Utc>>,
}

impl PersistentMemoryStore {
    /// Records one higher version of a source. A new version invalidates facts supported
    /// by previous observations of that source, even if its reported text is unchanged.
    /// Both the observation and any resulting status changes commit atomically.
    pub fn record_observation(
        &mut self,
        source_key: &str,
        version: i64,
        value: &str,
        source: EventSource,
    ) -> Result<Observation> {
        let source_key = normalized_label(source_key)?;
        if version < 1 {
            return Err(StoreError::InvalidInput(
                "observation version must be at least 1",
            ));
        }
        if value.trim().is_empty() {
            return Err(StoreError::InvalidInput(
                "observation value must not be empty",
            ));
        }
        let observation = Observation {
            id: Uuid::new_v4(),
            source_key: source_key.clone(),
            version,
            value: value.trim().to_owned(),
            event_id: Uuid::new_v4(),
            observed_at: Utc::now(),
        };
        let event = CognitiveEvent {
            id: observation.event_id,
            timestamp: observation.observed_at,
            kind: EventKind::Observation,
            source,
            content: observation.value.clone(),
            confidence: 1.0,
        };
        let tx = self.conn.transaction()?;
        let previous: Option<i64> = tx.query_row(
            "SELECT MAX(version) FROM observations WHERE source_key=?1",
            params![source_key],
            |row| row.get(0),
        )?;
        if previous.is_some_and(|current| version <= current) {
            return Err(StoreError::InvalidInput(
                "observation version must increase for this source",
            ));
        }
        tx.execute(
            "INSERT INTO events (id,event_json) VALUES (?1,?2)",
            params![event.id.to_string(), serde_json::to_string(&event)?],
        )?;
        tx.execute(
            "INSERT INTO observations (id,source_key,version,value,event_id,observed_at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                observation.id.to_string(),
                source_key,
                version,
                observation.value,
                observation.event_id.to_string(),
                observation.observed_at.to_rfc3339(),
            ],
        )?;

        // Invalidate any currently live fact that depends on an old version of this source.
        // Also includes DISPUTED facts: a dispute does not make its evidence current.
        let mut affected = Vec::new();
        {
            let mut statement = tx.prepare(
                "SELECT DISTINCT f.id,f.entity,f.attribute
                 FROM working_facts f
                 JOIN fact_evidence fe ON fe.fact_id=f.id
                 JOIN observations o ON o.id=fe.observation_id
                 WHERE o.source_key=?1 AND o.id<>?2 AND f.status<>'stale'",
            )?;
            let rows =
                statement.query_map(params![source_key, observation.id.to_string()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?;
            for row in rows {
                affected.push(row?);
            }
        }
        for (id, _, _) in &affected {
            tx.execute(
                "UPDATE working_facts SET status='stale',stale_at=?1 WHERE id=?2",
                params![observation.observed_at.to_rfc3339(), id],
            )?;
        }
        let mut keys = HashSet::new();
        for (_, entity, attribute) in affected {
            keys.insert((entity, attribute));
        }
        for (entity, attribute) in keys {
            refresh_disputes(&tx, &entity, &attribute)?;
        }
        tx.commit()?;
        Ok(observation)
    }

    /// A fact must cite at least one current observation. A derived fact is a user-entered
    /// claim *about* those observations, not a verified reasoning output.
    pub fn create_working_fact(
        &mut self,
        entity: &str,
        attribute: &str,
        value: &str,
        kind: FactKind,
        evidence_ids: &[Uuid],
    ) -> Result<WorkingFact> {
        let entity = normalized_label(entity)?;
        let attribute = normalized_label(attribute)?;
        if value.trim().is_empty() {
            return Err(StoreError::InvalidInput("fact value must not be empty"));
        }
        if evidence_ids.is_empty() {
            return Err(StoreError::InvalidInput(
                "fact requires supporting observations",
            ));
        }
        let unique: HashSet<Uuid> = evidence_ids.iter().copied().collect();
        if unique.len() != evidence_ids.len() {
            return Err(StoreError::InvalidInput("duplicate supporting observation"));
        }
        let tx = self.conn.transaction()?;
        for id in evidence_ids {
            let state: Option<(String, i64)> = tx
                .query_row(
                    "SELECT source_key,version FROM observations WHERE id=?1",
                    params![id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            let (source_key, version) = state.ok_or(StoreError::NotFound)?;
            let latest: i64 = tx.query_row(
                "SELECT MAX(version) FROM observations WHERE source_key=?1",
                params![source_key],
                |row| row.get(0),
            )?;
            if version != latest {
                return Err(StoreError::InvalidInput("supporting observation is stale"));
            }
        }
        let fact = WorkingFact {
            id: Uuid::new_v4(),
            entity,
            attribute,
            value: value.trim().to_owned(),
            kind,
            status: match kind {
                FactKind::Observed => FactStatus::Observed,
                FactKind::Derived => FactStatus::Derived,
            },
            evidence_ids: evidence_ids.to_vec(),
            created_at: Utc::now(),
            stale_at: None,
        };
        tx.execute(
            "INSERT INTO working_facts (id,entity,attribute,value,kind,status,created_at,stale_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,NULL)",
            params![
                fact.id.to_string(),
                fact.entity,
                fact.attribute,
                fact.value,
                fact.kind.as_str(),
                fact.kind.as_str(),
                fact.created_at.to_rfc3339(),
            ],
        )?;
        for id in evidence_ids {
            tx.execute(
                "INSERT INTO fact_evidence (fact_id,observation_id) VALUES (?1,?2)",
                params![fact.id.to_string(), id.to_string()],
            )?;
        }
        refresh_disputes(&tx, &fact.entity, &fact.attribute)?;
        tx.commit()?;
        self.working_fact(fact.id)?.ok_or(StoreError::NotFound)
    }

    pub fn observation(&self, id: Uuid) -> Result<Option<Observation>> {
        self.conn
            .query_row(
                "SELECT id,source_key,version,value,event_id,observed_at
                 FROM observations WHERE id=?1",
                params![id.to_string()],
                observation_from_row,
            )
            .optional()?
            .map(observation_from_raw)
            .transpose()
    }

    pub fn latest_observation(&self, source_key: &str) -> Result<Option<Observation>> {
        let source_key = normalized_label(source_key)?;
        self.conn
            .query_row(
                "SELECT id,source_key,version,value,event_id,observed_at
                 FROM observations WHERE source_key=?1 ORDER BY version DESC LIMIT 1",
                params![source_key],
                observation_from_row,
            )
            .optional()?
            .map(observation_from_raw)
            .transpose()
    }

    pub fn working_fact(&self, id: Uuid) -> Result<Option<WorkingFact>> {
        let raw = self
            .conn
            .query_row(
                "SELECT id,entity,attribute,value,kind,status,created_at,stale_at
                 FROM working_facts WHERE id=?1",
                params![id.to_string()],
                fact_from_row,
            )
            .optional()?;
        raw.map(|entry| self.fact_from_raw(entry)).transpose()
    }

    /// Includes stale and disputed facts so the human can inspect the complete working state.
    pub fn working_facts(&self, entity: &str) -> Result<Vec<WorkingFact>> {
        let entity = normalized_label(entity)?;
        let mut stmt = self.conn.prepare(
            "SELECT id,entity,attribute,value,kind,status,created_at,stale_at
             FROM working_facts WHERE entity=?1 ORDER BY rowid DESC",
        )?;
        let rows = stmt.query_map(params![entity], fact_from_row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(self.fact_from_raw(row?)?);
        }
        Ok(result)
    }

    /// Read the entire working-state projection for the visual workspace.
    /// An explicit empty-entity query is not passed to `working_facts`, which
    /// intentionally requires a valid entity label.
    pub fn all_working_facts(&self) -> Result<Vec<WorkingFact>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,entity,attribute,value,kind,status,created_at,stale_at
             FROM working_facts ORDER BY rowid DESC",
        )?;
        let rows = stmt.query_map([], fact_from_row)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(self.fact_from_raw(row?)?);
        }
        Ok(result)
    }

    pub fn observation_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM observations", [], |row| row.get(0))?)
    }

    pub fn working_fact_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM working_facts", [], |row| row.get(0))?)
    }

    fn fact_from_raw(&self, raw: RawFact) -> Result<WorkingFact> {
        let id = Uuid::parse_str(&raw.0)
            .map_err(|_| StoreError::InvalidInput("invalid stored fact UUID"))?;
        let mut stmt = self
            .conn
            .prepare("SELECT observation_id FROM fact_evidence WHERE fact_id=?1 ORDER BY rowid")?;
        let rows = stmt.query_map(params![id.to_string()], |row| row.get::<_, String>(0))?;
        let mut evidence_ids = Vec::new();
        for row in rows {
            evidence_ids.push(
                Uuid::parse_str(&row?)
                    .map_err(|_| StoreError::InvalidInput("invalid stored observation UUID"))?,
            );
        }
        Ok(WorkingFact {
            id,
            entity: raw.1,
            attribute: raw.2,
            value: raw.3,
            kind: FactKind::from_str(&raw.4)?,
            status: FactStatus::from_str(&raw.5)?,
            evidence_ids,
            created_at: parse_date(&raw.6)?,
            stale_at: raw.7.as_deref().map(parse_date).transpose()?,
        })
    }
}

type RawObservation = (String, String, i64, String, String, String);
type RawFact = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
);

fn observation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawObservation> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn observation_from_raw(raw: RawObservation) -> Result<Observation> {
    Ok(Observation {
        id: Uuid::parse_str(&raw.0)
            .map_err(|_| StoreError::InvalidInput("invalid stored observation UUID"))?,
        source_key: raw.1,
        version: raw.2,
        value: raw.3,
        event_id: Uuid::parse_str(&raw.4)
            .map_err(|_| StoreError::InvalidInput("invalid stored event UUID"))?,
        observed_at: parse_date(&raw.5)?,
    })
}

fn fact_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawFact> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
    ))
}

fn parse_date(raw: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(raw)
        .map_err(|_| StoreError::InvalidInput("invalid stored timestamp"))?
        .with_timezone(&Utc))
}

fn normalized_label(input: &str) -> Result<String> {
    let result = input.trim().to_lowercase();
    if result.is_empty() {
        return Err(StoreError::InvalidInput(
            "entity/attribute/source must not be empty",
        ));
    }
    Ok(result)
}

fn refresh_disputes(tx: &Transaction<'_>, entity: &str, attribute: &str) -> Result<()> {
    let distinct: i64 = tx.query_row(
        "SELECT COUNT(DISTINCT value) FROM working_facts
         WHERE entity=?1 AND attribute=?2 AND status<>'stale'",
        params![entity, attribute],
        |row| row.get(0),
    )?;
    if distinct > 1 {
        tx.execute(
            "UPDATE working_facts SET status='disputed'
             WHERE entity=?1 AND attribute=?2 AND status<>'stale'",
            params![entity, attribute],
        )?;
    } else {
        tx.execute(
            "UPDATE working_facts SET status=kind
             WHERE entity=?1 AND attribute=?2 AND status<>'stale'",
            params![entity, attribute],
        )?;
    }
    Ok(())
}
