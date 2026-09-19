use cognitive_core::{EventSource, FactKind, FactStatus, PersistentMemoryStore, StoreError};
use tempfile::tempdir;

#[test]
fn newer_source_revision_stales_only_dependent_facts_after_restart() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("working.sqlite3");
    let (old_fact, other_fact, old_observation, new_observation);
    {
        let mut store = PersistentMemoryStore::open(&path).unwrap();
        let old = store
            .record_observation("repo:main", 1, "commit A", EventSource::Tool("git".into()))
            .unwrap();
        let unrelated = store
            .record_observation(
                "calendar",
                1,
                "meeting at 4",
                EventSource::Tool("calendar".into()),
            )
            .unwrap();
        let supported = store
            .create_working_fact("project", "commit", "A", FactKind::Observed, &[old.id])
            .unwrap();
        let other = store
            .create_working_fact(
                "schedule",
                "meeting",
                "4pm",
                FactKind::Derived,
                &[unrelated.id],
            )
            .unwrap();
        assert_eq!(supported.status, FactStatus::Observed);
        assert_eq!(other.status, FactStatus::Derived);
        let updated = store
            .record_observation("repo:main", 2, "commit B", EventSource::Tool("git".into()))
            .unwrap();
        old_fact = supported.id;
        other_fact = other.id;
        old_observation = old.id;
        new_observation = updated.id;
        assert_eq!(
            store.working_fact(old_fact).unwrap().unwrap().status,
            FactStatus::Stale
        );
        assert!(store
            .working_fact(old_fact)
            .unwrap()
            .unwrap()
            .stale_at
            .is_some());
        assert_eq!(
            store.working_fact(other_fact).unwrap().unwrap().status,
            FactStatus::Derived
        );
    }
    let mut store = PersistentMemoryStore::open(&path).unwrap();
    let old = store.working_fact(old_fact).unwrap().unwrap();
    assert_eq!(old.status, FactStatus::Stale);
    assert_eq!(old.evidence_ids, vec![old_observation]);
    assert_eq!(
        store.latest_observation("repo:main").unwrap().unwrap().id,
        new_observation
    );
    assert!(matches!(
        store.create_working_fact(
            "project",
            "commit",
            "A",
            FactKind::Observed,
            &[old_observation]
        ),
        Err(StoreError::InvalidInput(_))
    ));
    let refreshed = store
        .create_working_fact(
            "project",
            "commit",
            "B",
            FactKind::Observed,
            &[new_observation],
        )
        .unwrap();
    assert_eq!(refreshed.status, FactStatus::Observed);
    assert_eq!(
        store.working_fact(other_fact).unwrap().unwrap().status,
        FactStatus::Derived
    );
}

#[test]
fn rejects_invalid_revisions_and_missing_evidence_without_partial_events() {
    let temp = tempdir().unwrap();
    let mut store = PersistentMemoryStore::open(temp.path().join("valid.db")).unwrap();
    assert!(store
        .record_observation(" ", 1, "data", EventSource::User)
        .is_err());
    assert!(store
        .record_observation("src", 0, "data", EventSource::User)
        .is_err());
    assert!(store
        .record_observation("src", 1, " ", EventSource::User)
        .is_err());
    assert_eq!(store.event_count().unwrap(), 0);
    let first = store
        .record_observation("src", 1, "valid", EventSource::User)
        .unwrap();
    assert!(store
        .record_observation("src", 1, "duplicate", EventSource::User)
        .is_err());
    assert!(store
        .record_observation("src", -1, "invalid", EventSource::User)
        .is_err());
    assert!(store
        .create_working_fact("project", "state", "ready", FactKind::Observed, &[])
        .is_err());
    assert!(store
        .create_working_fact(
            "project",
            "state",
            "ready",
            FactKind::Observed,
            &[first.id, first.id]
        )
        .is_err());
    assert!(store
        .create_working_fact(
            "project",
            "state",
            "ready",
            FactKind::Observed,
            &[uuid::Uuid::new_v4()]
        )
        .is_err());
    assert_eq!(store.event_count().unwrap(), 1);
    assert_eq!(store.observation_count().unwrap(), 1);
    assert_eq!(store.working_fact_count().unwrap(), 0);
}

#[test]
fn disputed_claims_reconcile_when_their_observation_goes_stale() {
    let temp = tempdir().unwrap();
    let mut store = PersistentMemoryStore::open(temp.path().join("dispute.db")).unwrap();
    let first_obs = store
        .record_observation("source-a", 1, "green", EventSource::User)
        .unwrap();
    let second_obs = store
        .record_observation("source-b", 1, "red", EventSource::User)
        .unwrap();
    let a = store
        .create_working_fact(
            "fruit",
            "color",
            "green",
            FactKind::Observed,
            &[first_obs.id],
        )
        .unwrap();
    let b = store
        .create_working_fact("fruit", "color", "red", FactKind::Derived, &[second_obs.id])
        .unwrap();
    assert_eq!(
        store.working_fact(a.id).unwrap().unwrap().status,
        FactStatus::Disputed
    );
    assert_eq!(
        store.working_fact(b.id).unwrap().unwrap().status,
        FactStatus::Disputed
    );
    store
        .record_observation("source-b", 2, "green", EventSource::User)
        .unwrap();
    assert_eq!(
        store.working_fact(b.id).unwrap().unwrap().status,
        FactStatus::Stale
    );
    assert_eq!(
        store.working_fact(a.id).unwrap().unwrap().status,
        FactStatus::Observed
    );
}

#[test]
fn multi_source_fact_stales_on_either_dependency_and_restores_from_backup() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("main.db");
    let backup = temp.path().join("backup.db");
    let mut store = PersistentMemoryStore::open(&path).unwrap();
    let a = store
        .record_observation("sensor:gas", 1, "11", EventSource::Tool("sensor".into()))
        .unwrap();
    let b = store
        .record_observation("sensor:temp", 1, "24", EventSource::Tool("sensor".into()))
        .unwrap();
    let fact = store
        .create_working_fact(
            "fruit",
            "quality",
            "uncertain",
            FactKind::Derived,
            &[a.id, b.id],
        )
        .unwrap();
    store
        .record_observation("sensor:temp", 2, "25", EventSource::Tool("sensor".into()))
        .unwrap();
    assert_eq!(
        store.working_fact(fact.id).unwrap().unwrap().status,
        FactStatus::Stale
    );
    store.backup(&backup).unwrap();
    drop(store);
    let restored = PersistentMemoryStore::open(&backup).unwrap();
    assert_eq!(
        restored.working_fact(fact.id).unwrap().unwrap().status,
        FactStatus::Stale
    );
    assert_eq!(
        restored
            .working_fact(fact.id)
            .unwrap()
            .unwrap()
            .evidence_ids
            .len(),
        2
    );
}

#[test]
fn existing_v02_memories_are_untouched_by_working_state_migration() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("legacy.db");
    let saved;
    {
        let mut store = PersistentMemoryStore::open(&path).unwrap();
        saved = store
            .create_memory(
                "Keep past project memory",
                cognitive_core::MemoryKind::Semantic,
                cognitive_core::EpistemicType::UserConfirmedFact,
                Some("project"),
            )
            .unwrap();
    }
    let mut store = PersistentMemoryStore::open(&path).unwrap();
    store
        .record_observation("repo", 1, "initial", EventSource::User)
        .unwrap();
    assert_eq!(
        store.active_memory(saved.memory.id).unwrap().memory.text,
        "Keep past project memory"
    );
    assert_eq!(store.memory_count().unwrap(), 1);
}
