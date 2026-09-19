use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    Observation,
    UserStatement,
    MemoryRecall,
    Inference,
    Hypothesis,
    Plan,
    ActionProposal,
    ActionOutcome,
    Reflection,
    MemorySuperseded,
    MemoryDeleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource {
    User,
    System,
    Tool(String),
    Model(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub kind: EventKind,
    pub source: EventSource,
    pub content: String,
    pub confidence: f32,
}

impl CognitiveEvent {
    pub fn new(
        kind: EventKind,
        source: EventSource,
        content: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            kind,
            source,
            content: content.into(),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}
