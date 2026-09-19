use cognitive_core::{EpistemicType, MemoryKind, PersistentMemoryStore, StoreError};
use tempfile::tempdir;

#[test]
fn remembers_after_process_reopen_with_original_source() {
    let temp = tempdir().unwrap();
    let db = temp.path().join("memory.sqlite3");
    let (memory_id, source_id);
    {
        let mut store = PersistentMemoryStore::open(&db).unwrap();
        let saved = store
            .create_memory(
                "EXOCORTEX experiment compares chatbot and kernel",
                MemoryKind::Semantic,
                EpistemicType::UserConfirmedFact,
                Some("experiment"),
            )
            .unwrap();
        memory_id = saved.memory.id;
        source_id = saved.memory.source_event_id;
    }
    {
        let store = PersistentMemoryStore::open(&db).unwrap();
        let recalled = store.recall("experiment").unwrap();
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].memory.id, memory_id);
        assert_eq!(recalled[0].memory.source_event_id, source_id);
        assert_eq!(
            recalled[0].memory.epistemic_type,
            EpistemicType::UserConfirmedFact
        );
        assert!(!recalled[0].memory.created_at.to_rfc3339().is_empty());
        let source = store.source(source_id).unwrap().unwrap();
        assert_eq!(source.content, recalled[0].memory.text);
    }
}

#[test]
fn all_memory_kinds_and_epistemic_types_round_trip() {
    let temp = tempdir().unwrap();
    let mut store = PersistentMemoryStore::open(temp.path().join("kinds.db")).unwrap();
    let kinds = [
        MemoryKind::Working,
        MemoryKind::Episodic,
        MemoryKind::Semantic,
        MemoryKind::Prospective,
    ];
    for kind in kinds {
        let saved = store
            .create_memory("Test record", kind.clone(), EpistemicType::Hypothesis, None)
            .unwrap();
        let retrieved = store.active_memory(saved.memory.id).unwrap();
        assert_eq!(retrieved.memory.kind, kind);
        assert_eq!(retrieved.memory.epistemic_type, EpistemicType::Hypothesis);
    }
    assert_eq!(store.memory_count().unwrap(), 4);
}

#[test]
fn correction_supersedes_and_keeps_new_provenance_after_reopen() {
    let temp = tempdir().unwrap();
    let db = temp.path().join("correction.db");
    let (old_id, new_id, new_source);
    {
        let mut store = PersistentMemoryStore::open(&db).unwrap();
        let old = store
            .create_memory(
                "Milestone V0.1",
                MemoryKind::Semantic,
                EpistemicType::Inference,
                Some("milestone"),
            )
            .unwrap();
        let new = store.correct(old.memory.id, "Milestone V0.2").unwrap();
        old_id = old.memory.id;
        new_id = new.memory.id;
        new_source = new.memory.source_event_id;
        assert!(matches!(
            store.active_memory(old_id),
            Err(StoreError::InactiveMemory)
        ));
    }
    let store = PersistentMemoryStore::open(&db).unwrap();
    let current = store.recall("Milestone").unwrap();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0].memory.id, new_id);
    assert_eq!(
        current[0].memory.epistemic_type,
        EpistemicType::UserConfirmedFact
    );
    assert_eq!(
        store.source(new_source).unwrap().unwrap().content,
        "Milestone V0.2"
    );
}

#[test]
fn different_claims_with_explicit_same_topic_are_flagged_not_auto_overwritten() {
    let temp = tempdir().unwrap();
    let mut store = PersistentMemoryStore::open(temp.path().join("conflict.db")).unwrap();
    let a = store
        .create_memory(
            "Deadline Monday",
            MemoryKind::Prospective,
            EpistemicType::UserConfirmedFact,
            Some("deadline"),
        )
        .unwrap();
    let b = store
        .create_memory(
            "Deadline Friday",
            MemoryKind::Prospective,
            EpistemicType::UserConfirmedFact,
            Some("deadline"),
        )
        .unwrap();
    let conflicts = store.possible_conflicts(b.memory.id).unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].other_id, a.memory.id);
    assert_eq!(store.recall("").unwrap().len(), 2);
    let no_topic = store
        .create_memory(
            "Deadline Sunday",
            MemoryKind::Prospective,
            EpistemicType::Prediction,
            None,
        )
        .unwrap();
    assert!(store
        .possible_conflicts(no_topic.memory.id)
        .unwrap()
        .is_empty());
}

#[test]
fn forget_redacts_correction_chain_and_source_and_survives_restart() {
    let temp = tempdir().unwrap();
    let db = temp.path().join("forget.db");
    let (old_id, new_id, source_id);
    {
        let mut store = PersistentMemoryStore::open(&db).unwrap();
        let old = store
            .create_memory(
                "private draft A",
                MemoryKind::Semantic,
                EpistemicType::UserConfirmedFact,
                Some("draft"),
            )
            .unwrap();
        source_id = old.memory.source_event_id;
        let new = store.correct(old.memory.id, "private draft B").unwrap();
        old_id = old.memory.id;
        new_id = new.memory.id;
        assert_eq!(store.forget(new_id).unwrap(), 2);
    }
    let store = PersistentMemoryStore::open(&db).unwrap();
    assert!(store.recall("private draft").unwrap().is_empty());
    assert!(matches!(
        store.active_memory(old_id),
        Err(StoreError::InactiveMemory)
    ));
    assert!(matches!(
        store.active_memory(new_id),
        Err(StoreError::InactiveMemory)
    ));
    assert_eq!(
        store.source(source_id).unwrap().unwrap().content,
        "[REDACTED BY USER]"
    );
}

#[test]
fn json_export_and_sqlite_backup_are_restorable() {
    let temp = tempdir().unwrap();
    let db = temp.path().join("main.db");
    let export = temp.path().join("export.json");
    let backup = temp.path().join("backup.db");
    let mut store = PersistentMemoryStore::open(&db).unwrap();
    let _ = store
        .create_memory(
            "Remember a project decision",
            MemoryKind::Semantic,
            EpistemicType::UserConfirmedFact,
            Some("decision"),
        )
        .unwrap();
    assert_eq!(store.export_json(&export).unwrap(), 1);
    assert_eq!(
        store.export_json(&export).unwrap_err().to_string(),
        StoreError::PathExists.to_string()
    );
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(&export).unwrap()).unwrap();
    assert_eq!(data["memories"].as_array().unwrap().len(), 1);
    assert_eq!(
        data["memories"][0]["source"]["content"],
        "Remember a project decision"
    );
    store.backup(&backup).unwrap();
    drop(store);
    let restored = PersistentMemoryStore::open(&backup).unwrap();
    assert_eq!(restored.recall("project decision").unwrap().len(), 1);
}

#[test]
fn invalid_inputs_do_not_create_partial_events() {
    let temp = tempdir().unwrap();
    let mut store = PersistentMemoryStore::open(temp.path().join("validation.db")).unwrap();
    assert!(store
        .create_memory("   ", MemoryKind::Working, EpistemicType::Observation, None)
        .is_err());
    assert!(store
        .create_memory(
            "A",
            MemoryKind::Semantic,
            EpistemicType::Inference,
            Some(" ")
        )
        .is_err());
    assert_eq!(store.event_count().unwrap(), 0);
    assert_eq!(store.memory_count().unwrap(), 0);
}
