use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionRisk {
    Internal,
    ReadOnly,
    Reversible,
    ExternalWrite,
    HighImpact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalState {
    NotRequired,
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionProposal {
    pub id: Uuid,
    pub action: String,
    pub summary: String,
    pub risk: ActionRisk,
    pub args: Value,
    pub approval_state: ApprovalState,
    pub created_at: DateTime<Utc>,
}

impl ActionProposal {
    pub fn new(
        action: impl Into<String>,
        summary: impl Into<String>,
        risk: ActionRisk,
        args: Value,
    ) -> Self {
        let approval_state = if Self::risk_requires_human_approval(&risk) {
            ApprovalState::Pending
        } else {
            ApprovalState::NotRequired
        };

        Self {
            id: Uuid::new_v4(),
            action: action.into(),
            summary: summary.into(),
            risk,
            args,
            approval_state,
            created_at: Utc::now(),
        }
    }

    pub fn requires_human_approval(&self) -> bool {
        Self::risk_requires_human_approval(&self.risk)
    }

    pub fn approve(&mut self) {
        if self.requires_human_approval() {
            self.approval_state = ApprovalState::Approved;
        }
    }

    pub fn reject(&mut self) {
        if self.requires_human_approval() {
            self.approval_state = ApprovalState::Rejected;
        }
    }

    fn risk_requires_human_approval(risk: &ActionRisk) -> bool {
        matches!(risk, ActionRisk::ExternalWrite | ActionRisk::HighImpact)
    }
}
