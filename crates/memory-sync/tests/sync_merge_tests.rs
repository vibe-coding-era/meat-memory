use memory_sync::{
    ApplyBatchResult, InMemoryReplicationEngine, MergeDecision, OplogEntry, OplogOperation,
    ReplicationEngine, SyncBatch, SyncCursor, SyncObjectKind, append_oplog_entry, merge_ops,
};

#[tokio::test]
async fn append_and_pull_returns_cursor_bounded_batches() {
    let engine = InMemoryReplicationEngine::default();

    let first = append_oplog_entry(
        &engine,
        OplogOperation::CreateObject,
        SyncObjectKind::Memory,
        "mem_sync_1",
        "node-a",
        "actor-a",
        0,
        1,
        serde_json::json!({"title":"one"}),
    )
    .await
    .unwrap();
    append_oplog_entry(
        &engine,
        OplogOperation::UpdateObject,
        SyncObjectKind::Memory,
        "mem_sync_1",
        "node-a",
        "actor-a",
        1,
        2,
        serde_json::json!({"title":"two"}),
    )
    .await
    .unwrap();

    let first_batch = engine
        .pull(SyncCursor {
            after_op_id: None,
            limit: 1,
        })
        .await
        .unwrap();
    let second_batch = engine
        .pull(SyncCursor {
            after_op_id: Some(first.op_id),
            limit: 10,
        })
        .await
        .unwrap();

    assert_eq!(first_batch.entries.len(), 1);
    assert_eq!(second_batch.entries.len(), 1);
    assert_eq!(second_batch.entries[0].next_version, 2);
}

#[test]
fn merge_reports_conflict_for_divergent_same_version() {
    let local = OplogEntry::new(
        OplogOperation::UpdateObject,
        SyncObjectKind::Memory,
        "mem_conflict",
        "node-local",
        "actor-local",
        1,
        2,
        serde_json::json!({"title":"left"}),
    );
    let incoming = OplogEntry::new(
        OplogOperation::UpdateObject,
        SyncObjectKind::Memory,
        "mem_conflict",
        "node-remote",
        "actor-remote",
        1,
        2,
        serde_json::json!({"title":"right"}),
    );

    let merged = merge_ops(&local, &incoming);
    assert_eq!(merged.decision, MergeDecision::Conflict);
    assert!(merged.conflict_reason.is_some());
}

#[tokio::test]
async fn apply_counts_applied_conflicted_and_skipped_entries() {
    let engine = InMemoryReplicationEngine::default();
    let existing = append_oplog_entry(
        &engine,
        OplogOperation::CreateObject,
        SyncObjectKind::Memory,
        "mem_apply",
        "node-a",
        "actor-a",
        0,
        1,
        serde_json::json!({"title":"initial"}),
    )
    .await
    .unwrap();

    let conflict = OplogEntry::new(
        OplogOperation::UpdateObject,
        SyncObjectKind::Memory,
        "mem_apply",
        "node-b",
        "actor-b",
        0,
        1,
        serde_json::json!({"title":"fork"}),
    );
    let forward = OplogEntry::new(
        OplogOperation::UpdateObject,
        SyncObjectKind::Memory,
        "mem_apply",
        "node-b",
        "actor-b",
        1,
        2,
        serde_json::json!({"title":"forward"}),
    );

    let result: ApplyBatchResult = engine
        .apply(SyncBatch {
            entries: vec![existing.clone(), conflict, forward],
            next_cursor: None,
        })
        .await
        .unwrap();

    assert_eq!(result.skipped, 1);
    assert_eq!(result.conflicts, 1);
    assert_eq!(result.applied, 1);
    assert_eq!(engine.entries().len(), 2);
}
