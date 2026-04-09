use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Mutex;
use time::OffsetDateTime;
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    Queued,
    InFlight,
    Acked,
    DeadLetter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OplogOperation {
    CreateObject,
    UpdateObject,
    DeleteObject,
    AttachEvidence,
    PublishMemory,
    MergeEntity,
    SplitEntity,
    RenderProjection,
    SyncAck,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncObjectKind {
    Artifact,
    Episode,
    Memory,
    Entity,
    Relation,
    Projection,
    Scope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OplogEntry {
    pub op_id: String,
    pub op_type: OplogOperation,
    pub object_kind: SyncObjectKind,
    pub object_id: String,
    pub source_node_id: String,
    pub actor_id: String,
    pub base_version: u64,
    pub next_version: u64,
    pub payload: Value,
    pub created_at: OffsetDateTime,
}

impl OplogEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        op_type: OplogOperation,
        object_kind: SyncObjectKind,
        object_id: impl Into<String>,
        source_node_id: impl Into<String>,
        actor_id: impl Into<String>,
        base_version: u64,
        next_version: u64,
        payload: Value,
    ) -> Self {
        Self {
            op_id: format!("op_{}", Ulid::new()),
            op_type,
            object_kind,
            object_id: object_id.into(),
            source_node_id: source_node_id.into(),
            actor_id: actor_id.into(),
            base_version,
            next_version,
            payload,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCursor {
    pub after_op_id: Option<String>,
    pub limit: usize,
}

impl Default for SyncCursor {
    fn default() -> Self {
        Self {
            after_op_id: None,
            limit: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncBatch {
    pub entries: Vec<OplogEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyBatchResult {
    pub applied: usize,
    pub conflicts: usize,
    pub skipped: usize,
    pub last_op_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeDecision {
    UseIncoming,
    KeepLocal,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeResult {
    pub decision: MergeDecision,
    pub merged_entry: OplogEntry,
    pub conflict_reason: Option<String>,
}

#[async_trait]
pub trait ReplicationEngine: Send + Sync {
    async fn append(&self, entry: OplogEntry) -> Result<()>;
    async fn pull(&self, cursor: SyncCursor) -> Result<SyncBatch>;
    async fn apply(&self, batch: SyncBatch) -> Result<ApplyBatchResult>;
}

#[derive(Debug, Default)]
pub struct InMemoryReplicationEngine {
    entries: Mutex<Vec<OplogEntry>>,
}

impl InMemoryReplicationEngine {
    pub fn entries(&self) -> Vec<OplogEntry> {
        self.entries.lock().unwrap().clone()
    }
}

#[async_trait]
impl ReplicationEngine for InMemoryReplicationEngine {
    async fn append(&self, entry: OplogEntry) -> Result<()> {
        self.entries.lock().unwrap().push(entry);
        Ok(())
    }

    async fn pull(&self, cursor: SyncCursor) -> Result<SyncBatch> {
        let entries = self.entries.lock().unwrap();
        let start = cursor
            .after_op_id
            .as_deref()
            .and_then(|target| entries.iter().position(|entry| entry.op_id == target))
            .map(|index| index + 1)
            .unwrap_or(0);
        let limit = cursor.limit.max(1);
        let slice = entries
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next_cursor = slice.last().map(|entry| entry.op_id.clone());

        Ok(SyncBatch {
            entries: slice,
            next_cursor,
        })
    }

    async fn apply(&self, batch: SyncBatch) -> Result<ApplyBatchResult> {
        let mut entries = self.entries.lock().unwrap();
        let mut applied = 0usize;
        let mut conflicts = 0usize;
        let mut skipped = 0usize;
        let mut last_op_id = None;

        for incoming in batch.entries {
            last_op_id = Some(incoming.op_id.clone());

            if entries
                .iter()
                .any(|existing| existing.op_id == incoming.op_id)
            {
                skipped += 1;
                continue;
            }

            if let Some(existing) = entries
                .iter()
                .rev()
                .find(|existing| {
                    existing.object_kind == incoming.object_kind
                        && existing.object_id == incoming.object_id
                })
                .cloned()
            {
                match merge_ops(&existing, &incoming) {
                    MergeResult {
                        decision: MergeDecision::UseIncoming,
                        ..
                    } => {
                        entries.push(incoming);
                        applied += 1;
                    }
                    MergeResult {
                        decision: MergeDecision::KeepLocal,
                        ..
                    } => {
                        skipped += 1;
                    }
                    MergeResult {
                        decision: MergeDecision::Conflict,
                        ..
                    } => {
                        conflicts += 1;
                    }
                }
            } else {
                entries.push(incoming);
                applied += 1;
            }
        }

        Ok(ApplyBatchResult {
            applied,
            conflicts,
            skipped,
            last_op_id,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn append_oplog_entry(
    engine: &(impl ReplicationEngine + ?Sized),
    op_type: OplogOperation,
    object_kind: SyncObjectKind,
    object_id: impl Into<String>,
    source_node_id: impl Into<String>,
    actor_id: impl Into<String>,
    base_version: u64,
    next_version: u64,
    payload: Value,
) -> Result<OplogEntry> {
    let entry = OplogEntry::new(
        op_type,
        object_kind,
        object_id,
        source_node_id,
        actor_id,
        base_version,
        next_version,
        payload,
    );
    engine.append(entry.clone()).await?;
    Ok(entry)
}

pub fn merge_ops(local: &OplogEntry, incoming: &OplogEntry) -> MergeResult {
    if local.object_kind != incoming.object_kind || local.object_id != incoming.object_id {
        return MergeResult {
            decision: MergeDecision::Conflict,
            merged_entry: local.clone(),
            conflict_reason: Some("object identity mismatch".to_string()),
        };
    }

    if local.op_type == incoming.op_type
        && local.base_version == incoming.base_version
        && local.next_version == incoming.next_version
        && local.payload == incoming.payload
    {
        return MergeResult {
            decision: MergeDecision::KeepLocal,
            merged_entry: local.clone(),
            conflict_reason: None,
        };
    }

    if incoming.base_version >= local.next_version && incoming.next_version > local.next_version {
        return MergeResult {
            decision: MergeDecision::UseIncoming,
            merged_entry: incoming.clone(),
            conflict_reason: None,
        };
    }

    if local.base_version >= incoming.next_version && local.next_version > incoming.next_version {
        return MergeResult {
            decision: MergeDecision::KeepLocal,
            merged_entry: local.clone(),
            conflict_reason: None,
        };
    }

    MergeResult {
        decision: MergeDecision::Conflict,
        merged_entry: local.clone(),
        conflict_reason: Some(format!(
            "divergent versions local={} incoming={}",
            local.next_version, incoming.next_version
        )),
    }
}

pub fn can_retry(state: SyncState) -> bool {
    matches!(state, SyncState::Queued | SyncState::InFlight)
}

#[cfg(test)]
mod tests {
    use super::{
        InMemoryReplicationEngine, MergeDecision, OplogEntry, OplogOperation, SyncCursor,
        SyncObjectKind, SyncState, append_oplog_entry, can_retry, merge_ops,
    };
    use crate::{ReplicationEngine, SyncBatch};
    use time::OffsetDateTime;

    #[test]
    fn oplog_entry_new_sets_metadata_and_identity() {
        let before = OffsetDateTime::now_utc();
        let entry = OplogEntry::new(
            OplogOperation::CreateObject,
            SyncObjectKind::Memory,
            "mem_meta",
            "node-a",
            "actor-a",
            0,
            1,
            serde_json::json!({"title":"hello"}),
        );
        let after = OffsetDateTime::now_utc();

        assert!(entry.op_id.starts_with("op_"));
        assert_eq!(entry.object_id, "mem_meta");
        assert_eq!(entry.source_node_id, "node-a");
        assert_eq!(entry.actor_id, "actor-a");
        assert_eq!(entry.base_version, 0);
        assert_eq!(entry.next_version, 1);
        assert!(entry.created_at >= before);
        assert!(entry.created_at <= after);
    }

    #[test]
    fn sync_cursor_defaults_to_first_page_of_hundred() {
        let cursor = SyncCursor::default();

        assert_eq!(cursor.after_op_id, None);
        assert_eq!(cursor.limit, 100);
    }

    #[test]
    fn only_retry_active_states() {
        assert!(can_retry(SyncState::Queued));
        assert!(can_retry(SyncState::InFlight));
        assert!(!can_retry(SyncState::Acked));
        assert!(!can_retry(SyncState::DeadLetter));
    }

    #[test]
    fn merge_prefers_incoming_forward_progress() {
        let local = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_1",
            "node-local",
            "actor-local",
            1,
            2,
            serde_json::json!({"title":"old"}),
        );
        let incoming = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_1",
            "node-remote",
            "actor-remote",
            2,
            3,
            serde_json::json!({"title":"new"}),
        );

        let merged = merge_ops(&local, &incoming);
        assert_eq!(merged.decision, MergeDecision::UseIncoming);
        assert_eq!(merged.merged_entry.next_version, 3);
    }

    #[test]
    fn merge_keeps_local_for_identical_entries() {
        let local = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_same",
            "node-local",
            "actor-local",
            1,
            2,
            serde_json::json!({"title":"same"}),
        );
        let incoming = local.clone();

        let merged = merge_ops(&local, &incoming);

        assert_eq!(merged.decision, MergeDecision::KeepLocal);
        assert_eq!(merged.conflict_reason, None);
        assert_eq!(merged.merged_entry.op_id, local.op_id);
    }

    #[test]
    fn merge_keeps_local_when_local_is_ahead() {
        let local = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_local_ahead",
            "node-local",
            "actor-local",
            3,
            5,
            serde_json::json!({"title":"newer-local"}),
        );
        let incoming = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_local_ahead",
            "node-remote",
            "actor-remote",
            1,
            2,
            serde_json::json!({"title":"older-remote"}),
        );

        let merged = merge_ops(&local, &incoming);

        assert_eq!(merged.decision, MergeDecision::KeepLocal);
        assert_eq!(merged.merged_entry.next_version, 5);
    }

    #[test]
    fn merge_reports_conflict_for_object_identity_mismatch() {
        let local = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_one",
            "node-local",
            "actor-local",
            1,
            2,
            serde_json::json!({"title":"left"}),
        );
        let incoming = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Entity,
            "ent_one",
            "node-remote",
            "actor-remote",
            1,
            2,
            serde_json::json!({"title":"right"}),
        );

        let merged = merge_ops(&local, &incoming);

        assert_eq!(merged.decision, MergeDecision::Conflict);
        assert_eq!(
            merged.conflict_reason.as_deref(),
            Some("object identity mismatch")
        );
    }

    #[tokio::test]
    async fn append_helper_persists_into_engine() {
        let engine = InMemoryReplicationEngine::default();
        let entry = append_oplog_entry(
            &engine,
            OplogOperation::CreateObject,
            SyncObjectKind::Memory,
            "mem_helper",
            "node-a",
            "actor-a",
            0,
            1,
            serde_json::json!({"title":"hello"}),
        )
        .await
        .unwrap();

        assert_eq!(engine.entries().len(), 1);
        assert_eq!(engine.entries()[0].op_id, entry.op_id);
    }

    #[tokio::test]
    async fn pull_uses_default_start_when_cursor_is_missing_and_limit_is_zero() {
        let engine = InMemoryReplicationEngine::default();
        let first = append_oplog_entry(
            &engine,
            OplogOperation::CreateObject,
            SyncObjectKind::Memory,
            "mem_pull_1",
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
            "mem_pull_1",
            "node-a",
            "actor-a",
            1,
            2,
            serde_json::json!({"title":"two"}),
        )
        .await
        .unwrap();

        let batch = engine
            .pull(SyncCursor {
                after_op_id: Some("missing-op".to_string()),
                limit: 0,
            })
            .await
            .unwrap();

        assert_eq!(batch.entries.len(), 1);
        assert_eq!(batch.entries[0].op_id, first.op_id);
        assert_eq!(batch.next_cursor.as_deref(), Some(first.op_id.as_str()));
    }

    #[tokio::test]
    async fn apply_accepts_new_entries_and_tracks_last_op_id() {
        let engine = InMemoryReplicationEngine::default();
        let first = OplogEntry::new(
            OplogOperation::CreateObject,
            SyncObjectKind::Memory,
            "mem_apply_new",
            "node-a",
            "actor-a",
            0,
            1,
            serde_json::json!({"title":"first"}),
        );
        let second = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_apply_new",
            "node-a",
            "actor-a",
            1,
            2,
            serde_json::json!({"title":"second"}),
        );

        let result = engine
            .apply(SyncBatch {
                entries: vec![first.clone(), second.clone()],
                next_cursor: None,
            })
            .await
            .unwrap();

        assert_eq!(result.applied, 2);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.conflicts, 0);
        assert_eq!(result.last_op_id.as_deref(), Some(second.op_id.as_str()));
        assert_eq!(engine.entries().len(), 2);
    }
}
