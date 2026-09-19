use serde_json::Value;
use uuid::Uuid;

use crate::{
    ActionProposal, ActionRisk, ApprovalState, CognitiveEvent, EpistemicType, EventKind,
    EventSource, MemoryKind, MemoryRecord,
};

#[derive(Debug, Default)]
pub struct CognitiveKernel {
    events: Vec<CognitiveEvent>,
    memories: Vec<MemoryRecord>,
    actions: Vec<ActionProposal>,
}

impl CognitiveKernel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_user(&mut self, content: impl Into<String>) -> CognitiveEvent {
        let event = CognitiveEvent::new(EventKind::UserStatement, EventSource::User, content, 1.0);
        self.events.push(event.clone());
        event
    }

    pub fn remember(
        &mut self,
        text: impl Into<String>,
        kind: MemoryKind,
        epistemic_type: EpistemicType,
        source_event_id: Uuid,
    ) -> MemoryRecord {
        let memory = MemoryRecord::new(text, kind, epistemic_type, source_event_id);
        self.memories.push(memory.clone());
        memory
    }

    pub fn recall(&self, query: &str) -> Vec<&MemoryRecord> {
        let needle = query.trim().to_lowercase();

        self.memories
            .iter()
            .filter(|memory| needle.is_empty() || memory.text.to_lowercase().contains(&needle))
            .collect()
    }

    pub fn propose_action(
        &mut self,
        action: impl Into<String>,
        summary: impl Into<String>,
        risk: ActionRisk,
        args: Value,
    ) -> ActionProposal {
        let proposal = ActionProposal::new(action, summary, risk, args);

        let event = CognitiveEvent::new(
            EventKind::ActionProposal,
            EventSource::System,
            proposal.summary.clone(),
            1.0,
        );

        self.events.push(event);
        self.actions.push(proposal.clone());
        proposal
    }

    pub fn approve_action(&mut self, id: Uuid) -> Option<ActionProposal> {
        let action = self.actions.iter_mut().find(|action| action.id == id)?;
        action.approve();
        Some(action.clone())
    }

    pub fn reject_action(&mut self, id: Uuid) -> Option<ActionProposal> {
        let action = self.actions.iter_mut().find(|action| action.id == id)?;
        action.reject();
        Some(action.clone())
    }

    pub fn pending_actions(&self) -> Vec<&ActionProposal> {
        self.actions
            .iter()
            .filter(|action| action.approval_state == ApprovalState::Pending)
            .collect()
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    pub fn memory_count(&self) -> usize {
        self.memories.len()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn confirmed_memory_preserves_epistemic_status_and_provenance() {
        let mut kernel = CognitiveKernel::new();
        let event = kernel.observe_user("The first experiment compares four conditions.");
        let memory = kernel.remember(
            event.content.clone(),
            MemoryKind::Semantic,
            EpistemicType::UserConfirmedFact,
            event.id,
        );

        assert_eq!(memory.epistemic_type, EpistemicType::UserConfirmedFact);
        assert_eq!(memory.source_event_id, event.id);
        assert_eq!(memory.provenance.len(), 1);
        assert_eq!(kernel.memory_count(), 1);
    }

    #[test]
    fn external_write_requires_human_approval() {
        let mut kernel = CognitiveKernel::new();
        let proposal = kernel.propose_action(
            "send_email",
            "Send experiment summary to the professor",
            ActionRisk::ExternalWrite,
            json!({"to": "professor@example.com"}),
        );

        assert!(proposal.requires_human_approval());
        assert_eq!(proposal.approval_state, ApprovalState::Pending);
        assert_eq!(kernel.pending_actions().len(), 1);

        let approved = kernel
            .approve_action(proposal.id)
            .expect("proposal should exist");

        assert_eq!(approved.approval_state, ApprovalState::Approved);
        assert!(kernel.pending_actions().is_empty());
    }

    #[test]
    fn internal_action_does_not_require_human_approval() {
        let mut kernel = CognitiveKernel::new();
        let proposal = kernel.propose_action(
            "summarize_memory",
            "Create an internal summary",
            ActionRisk::Internal,
            json!({}),
        );

        assert!(!proposal.requires_human_approval());
        assert_eq!(proposal.approval_state, ApprovalState::NotRequired);
    }
}
