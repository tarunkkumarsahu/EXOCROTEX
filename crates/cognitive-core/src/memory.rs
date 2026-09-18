use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Working,
    Episodic,
    Semantic,
    Prospective,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpistemicType {
    Observation,
    UserConfirmedFact,
    Inference,
    Hypothesis,
    Prediction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRef {
    pub source_type: String,
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: Uuid,
    pub text: String,
    pub kind: MemoryKind,
    pub epistemic_type: EpistemicType,
    pub source_event_id: Uuid,
    pub provenance: Vec<ProvenanceRef>,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub superseded_by: Option<Uuid>,
}

impl MemoryRecord {
    pub fn new(
        text: impl Into<String>,
        kind: MemoryKind,
        epistemic_type: EpistemicType,
        source_event_id: Uuid,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            text: text.into(),
            kind,
            epistemic_type,
            source_event_id,
            provenance: vec![ProvenanceRef {
                source_type: "cognitive_event".to_string(),
                source_id: source_event_id.to_string(),
            }],
            importance: 0.5,
            created_at: now,
            valid_from: now,
            valid_until: None,
            superseded_by: None,
        }
    }
}
