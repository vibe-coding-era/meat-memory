use anyhow::Result;
use async_trait::async_trait;
use memory_domain::{Artifact, DocumentConflictState, DocumentSyncState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub conflict_id: String,
    pub local_entry: OplogEntry,
    pub incoming_entry: OplogEntry,
    pub reason: String,
    pub detected_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatus {
    pub entry_count: usize,
    pub conflict_count: usize,
    pub last_op_id: Option<String>,
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
    conflicts: Mutex<Vec<ConflictRecord>>,
}

impl InMemoryReplicationEngine {
    pub fn entries(&self) -> Vec<OplogEntry> {
        self.entries.lock().unwrap().clone()
    }

    pub fn conflicts(&self) -> Vec<ConflictRecord> {
        self.conflicts.lock().unwrap().clone()
    }

    pub fn status(&self) -> SyncStatus {
        let entries = self.entries.lock().unwrap();
        let conflicts = self.conflicts.lock().unwrap();
        SyncStatus {
            entry_count: entries.len(),
            conflict_count: conflicts.len(),
            last_op_id: entries.last().map(|entry| entry.op_id.clone()),
        }
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
        Ok(pull_entries(&entries, cursor))
    }

    async fn apply(&self, batch: SyncBatch) -> Result<ApplyBatchResult> {
        let mut entries = self.entries.lock().unwrap();
        let mut conflicts = self.conflicts.lock().unwrap();
        Ok(apply_entries(&mut entries, &mut conflicts, batch))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
struct PersistentReplicationState {
    entries: Vec<OplogEntry>,
    conflicts: Vec<ConflictRecord>,
}

#[derive(Debug)]
pub struct FileReplicationEngine {
    path: PathBuf,
    state: Mutex<PersistentReplicationState>,
}

impl FileReplicationEngine {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let state = load_state(&path)?;
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub fn entries(&self) -> Vec<OplogEntry> {
        self.state.lock().unwrap().entries.clone()
    }

    pub fn conflicts(&self) -> Vec<ConflictRecord> {
        self.state.lock().unwrap().conflicts.clone()
    }

    pub fn status(&self) -> SyncStatus {
        let state = self.state.lock().unwrap();
        SyncStatus {
            entry_count: state.entries.len(),
            conflict_count: state.conflicts.len(),
            last_op_id: state.entries.last().map(|entry| entry.op_id.clone()),
        }
    }

    fn flush(&self, state: &PersistentReplicationState) -> Result<()> {
        persist_state(&self.path, state)
    }
}

#[async_trait]
impl ReplicationEngine for FileReplicationEngine {
    async fn append(&self, entry: OplogEntry) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        state.entries.push(entry);
        self.flush(&state)
    }

    async fn pull(&self, cursor: SyncCursor) -> Result<SyncBatch> {
        let state = self.state.lock().unwrap();
        Ok(pull_entries(&state.entries, cursor))
    }

    async fn apply(&self, batch: SyncBatch) -> Result<ApplyBatchResult> {
        let mut state = self.state.lock().unwrap();
        let result = apply_state(&mut state, batch);
        self.flush(&state)?;
        Ok(result)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDocumentSnapshot {
    pub canonical_uri: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalProjectDocumentDraft {
    pub canonical_uri: String,
    pub local_path: PathBuf,
    pub title: String,
    pub content_text: String,
    pub content_hash: String,
    pub sync_state: DocumentSyncState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingProjectDocument {
    pub canonical_uri: String,
    pub sync_state: DocumentSyncState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDocumentConflictInput {
    pub canonical_uri: String,
    pub base_content_hash: Option<String>,
    pub indexed_content_hash: Option<String>,
    pub local_content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDocumentConflictReport {
    pub canonical_uri: String,
    pub sync_state: DocumentSyncState,
    pub conflict_state: DocumentConflictState,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalProjectDocumentSyncPlan {
    pub root: PathBuf,
    pub documents: Vec<LocalProjectDocumentDraft>,
    pub missing: Vec<MissingProjectDocument>,
    pub conflicts: Vec<ProjectDocumentConflictReport>,
}

#[derive(Debug, Clone)]
pub struct LocalProjectDocumentSyncEngine {
    root: PathBuf,
    extensions: BTreeSet<String>,
    excluded_dir_names: BTreeSet<String>,
}

impl LocalProjectDocumentSyncEngine {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_extensions(root, ["md", "markdown", "txt"])
    }

    pub fn with_extensions<I, S>(root: impl Into<PathBuf>, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let extensions = extensions
            .into_iter()
            .map(|extension| normalize_extension(&extension.into()))
            .filter(|extension| !extension.is_empty())
            .collect::<BTreeSet<_>>();
        Self {
            root: root.into(),
            extensions,
            excluded_dir_names: BTreeSet::new(),
        }
    }

    pub fn with_excluded_dir_names<I, S>(mut self, excluded_dir_names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.excluded_dir_names = excluded_dir_names
            .into_iter()
            .map(|name| name.into())
            .filter(|name| !name.trim().is_empty())
            .collect();
        self
    }

    pub fn scan(
        &self,
        previous: &[ProjectDocumentSnapshot],
    ) -> Result<LocalProjectDocumentSyncPlan> {
        let root = self.root.canonicalize()?;
        let previous_by_uri = previous
            .iter()
            .map(|snapshot| {
                (
                    snapshot.canonical_uri.clone(),
                    snapshot.content_hash.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::new();
        let mut documents = Vec::new();

        for path in collect_document_paths(&root, &self.extensions, &self.excluded_dir_names)? {
            let content_text = fs::read_to_string(&path)?;
            let content_hash = Artifact::compute_content_hash(&content_text);
            let canonical_uri = canonical_file_uri(&path);
            seen.insert(canonical_uri.clone());
            let sync_state = match previous_by_uri.get(&canonical_uri) {
                Some(previous_hash) if previous_hash == &content_hash => DocumentSyncState::Clean,
                Some(_) => DocumentSyncState::Changed,
                None => DocumentSyncState::Changed,
            };
            documents.push(LocalProjectDocumentDraft {
                title: document_title(&path, &content_text),
                canonical_uri,
                local_path: path,
                content_text,
                content_hash,
                sync_state,
            });
        }

        let missing = previous
            .iter()
            .filter(|snapshot| !seen.contains(&snapshot.canonical_uri))
            .map(|snapshot| MissingProjectDocument {
                canonical_uri: snapshot.canonical_uri.clone(),
                sync_state: DocumentSyncState::Missing,
            })
            .collect::<Vec<_>>();
        let conflicts = build_conflict_reports(previous, &documents, &missing);

        Ok(LocalProjectDocumentSyncPlan {
            root,
            documents,
            missing,
            conflicts,
        })
    }
}

pub fn classify_project_document_conflict(
    input: ProjectDocumentConflictInput,
) -> ProjectDocumentConflictReport {
    let base = input.base_content_hash.as_deref();
    let indexed = input.indexed_content_hash.as_deref();
    let local = input.local_content_hash.as_deref();

    let (sync_state, conflict_state, reason) = match (base, indexed, local) {
        (None, None, None) => (
            DocumentSyncState::Missing,
            DocumentConflictState::None,
            Some("document is absent locally and has no indexed baseline".to_string()),
        ),
        (None, _, Some(_)) => (
            DocumentSyncState::Changed,
            DocumentConflictState::LocalChanged,
            None,
        ),
        (Some(base), Some(indexed), Some(local)) if base == indexed && base == local => {
            (DocumentSyncState::Clean, DocumentConflictState::None, None)
        }
        (Some(base), Some(indexed), None) if base == indexed => (
            DocumentSyncState::Deleted,
            DocumentConflictState::None,
            None,
        ),
        (Some(base), Some(indexed), None) if base != indexed => (
            DocumentSyncState::Conflicted,
            DocumentConflictState::BothChanged,
            Some("local document was deleted while indexed document changed".to_string()),
        ),
        (Some(_), Some(_), None) => (
            DocumentSyncState::Conflicted,
            DocumentConflictState::BothChanged,
            Some(
                "local document was deleted while indexed document state is ambiguous".to_string(),
            ),
        ),
        (Some(base), Some(indexed), Some(local)) if base == indexed && base != local => (
            DocumentSyncState::Changed,
            DocumentConflictState::LocalChanged,
            None,
        ),
        (Some(base), Some(indexed), Some(local)) if base == local && base != indexed => (
            DocumentSyncState::Changed,
            DocumentConflictState::RemoteChanged,
            None,
        ),
        (Some(_), Some(indexed), Some(local)) if indexed == local => (
            DocumentSyncState::Changed,
            DocumentConflictState::None,
            Some("local and indexed content converged after baseline".to_string()),
        ),
        (Some(_), Some(_), Some(_)) => (
            DocumentSyncState::Conflicted,
            DocumentConflictState::BothChanged,
            Some("local and indexed document changed differently".to_string()),
        ),
        (Some(_), None, None) => (
            DocumentSyncState::Deleted,
            DocumentConflictState::None,
            None,
        ),
        (Some(base), None, Some(local)) if base == local => {
            (DocumentSyncState::Clean, DocumentConflictState::None, None)
        }
        (Some(_), None, Some(_)) => (
            DocumentSyncState::Changed,
            DocumentConflictState::LocalChanged,
            None,
        ),
        (None, Some(_), None) => (
            DocumentSyncState::Changed,
            DocumentConflictState::RemoteChanged,
            None,
        ),
    };

    ProjectDocumentConflictReport {
        canonical_uri: input.canonical_uri,
        sync_state,
        conflict_state,
        reason,
    }
}

fn build_conflict_reports(
    previous: &[ProjectDocumentSnapshot],
    documents: &[LocalProjectDocumentDraft],
    missing: &[MissingProjectDocument],
) -> Vec<ProjectDocumentConflictReport> {
    let local_by_uri = documents
        .iter()
        .map(|document| {
            (
                document.canonical_uri.clone(),
                document.content_hash.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let missing_uris = missing
        .iter()
        .map(|document| document.canonical_uri.as_str())
        .collect::<BTreeSet<_>>();

    previous
        .iter()
        .filter_map(|snapshot| {
            let report = classify_project_document_conflict(ProjectDocumentConflictInput {
                canonical_uri: snapshot.canonical_uri.clone(),
                base_content_hash: Some(snapshot.content_hash.clone()),
                indexed_content_hash: Some(snapshot.content_hash.clone()),
                local_content_hash: local_by_uri.get(&snapshot.canonical_uri).cloned(),
            });
            if matches!(report.sync_state, DocumentSyncState::Clean)
                && !missing_uris.contains(snapshot.canonical_uri.as_str())
            {
                None
            } else {
                Some(report)
            }
        })
        .collect()
}

fn collect_document_paths(
    root: &Path,
    extensions: &BTreeSet<String>,
    excluded_dir_names: &BTreeSet<String>,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_document_paths_into(root, extensions, excluded_dir_names, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_document_paths_into(
    current: &Path,
    extensions: &BTreeSet<String>,
    excluded_dir_names: &BTreeSet<String>,
    output: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with('.') || excluded_dir_names.contains(name))
                .unwrap_or(false)
            {
                continue;
            }
            collect_document_paths_into(&path, extensions, excluded_dir_names, output)?;
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(normalize_extension)
                .map(|extension| extensions.contains(&extension))
                .unwrap_or(false)
        {
            output.push(path.canonicalize()?);
        }
    }
    Ok(())
}

fn canonical_file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

fn document_title(path: &Path, content_text: &str) -> String {
    content_text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .filter(|line| !line.is_empty())
        .or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn normalize_extension(extension: &str) -> String {
    extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
}

fn pull_entries(entries: &[OplogEntry], cursor: SyncCursor) -> SyncBatch {
    let start = cursor
        .after_op_id
        .as_deref()
        .and_then(|target| entries.iter().position(|entry| entry.op_id == target))
        .map(|index| index + 1)
        .unwrap_or(0);
    let limit = if cursor.limit == 0 { 100 } else { cursor.limit };
    let slice = entries
        .iter()
        .skip(start)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let next_cursor = slice.last().map(|entry| entry.op_id.clone());

    SyncBatch {
        entries: slice,
        next_cursor,
    }
}

fn apply_entries(
    entries: &mut Vec<OplogEntry>,
    conflicts: &mut Vec<ConflictRecord>,
    batch: SyncBatch,
) -> ApplyBatchResult {
    let mut applied = 0usize;
    let mut conflict_count = 0usize;
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
                } => skipped += 1,
                MergeResult {
                    decision: MergeDecision::Conflict,
                    conflict_reason,
                    ..
                } => {
                    conflicts.push(ConflictRecord {
                        conflict_id: format!("conf_{}", Ulid::new()),
                        local_entry: existing,
                        incoming_entry: incoming,
                        reason: conflict_reason.unwrap_or_else(|| "unknown conflict".to_string()),
                        detected_at: OffsetDateTime::now_utc(),
                    });
                    conflict_count += 1;
                }
            }
        } else {
            entries.push(incoming);
            applied += 1;
        }
    }

    ApplyBatchResult {
        applied,
        conflicts: conflict_count,
        skipped,
        last_op_id,
    }
}

fn apply_state(state: &mut PersistentReplicationState, batch: SyncBatch) -> ApplyBatchResult {
    apply_entries(&mut state.entries, &mut state.conflicts, batch)
}

fn load_state(path: &Path) -> Result<PersistentReplicationState> {
    match fs::read_to_string(path) {
        Ok(raw) => Ok(serde_json::from_str(&raw)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistentReplicationState::default())
        }
        Err(error) => Err(error.into()),
    }
}

fn persist_state(path: &Path, state: &PersistentReplicationState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        FileReplicationEngine, InMemoryReplicationEngine, LocalProjectDocumentSyncEngine,
        MergeDecision, OplogEntry, OplogOperation, ProjectDocumentConflictInput,
        ProjectDocumentSnapshot, SyncCursor, SyncObjectKind, SyncState, append_oplog_entry,
        can_retry, canonical_file_uri, classify_project_document_conflict, merge_ops,
    };
    use crate::{ReplicationEngine, SyncBatch};
    use memory_domain::{Artifact, DocumentConflictState, DocumentSyncState};
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
                after_op_id: Some("missing".to_string()),
                limit: 0,
            })
            .await
            .unwrap();

        assert_eq!(batch.entries.len(), 2);
        assert_eq!(batch.entries[0].op_id, first.op_id);
        assert_eq!(
            batch.next_cursor,
            batch.entries.last().map(|entry| entry.op_id.clone())
        );
    }

    #[tokio::test]
    async fn apply_accepts_new_entries_and_tracks_conflicts() {
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

        let result = engine
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
        assert_eq!(engine.conflicts().len(), 1);
        assert_eq!(engine.status().last_op_id, result.last_op_id);
    }

    #[tokio::test]
    async fn file_engine_persists_entries_and_conflicts() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("sync-state.json");
        let engine = FileReplicationEngine::open(&path).unwrap();
        let existing = append_oplog_entry(
            &engine,
            OplogOperation::CreateObject,
            SyncObjectKind::Memory,
            "mem_file",
            "node-a",
            "actor-a",
            0,
            1,
            serde_json::json!({"title":"v1"}),
        )
        .await
        .unwrap();

        let conflict = OplogEntry::new(
            OplogOperation::UpdateObject,
            SyncObjectKind::Memory,
            "mem_file",
            "node-b",
            "actor-b",
            0,
            1,
            serde_json::json!({"title":"fork"}),
        );

        let result = engine
            .apply(SyncBatch {
                entries: vec![existing, conflict],
                next_cursor: None,
            })
            .await
            .unwrap();

        assert_eq!(result.skipped, 1);
        assert_eq!(result.conflicts, 1);
        assert_eq!(engine.status().conflict_count, 1);

        let reopened = FileReplicationEngine::open(&path).unwrap();
        assert_eq!(reopened.entries().len(), 1);
        assert_eq!(reopened.conflicts().len(), 1);
    }

    #[test]
    fn local_project_document_sync_scans_changed_clean_and_missing_documents() {
        let tempdir = tempfile::tempdir().unwrap();
        let docs = tempdir.path().join("docs");
        std::fs::create_dir_all(docs.join("nested")).unwrap();
        std::fs::write(docs.join("README.md"), "# Project README\nhello").unwrap();
        std::fs::write(docs.join("nested").join("guide.txt"), "Guide\nbody").unwrap();
        std::fs::write(docs.join("skip.json"), "{}").unwrap();

        let readme_uri = canonical_file_uri(&docs.join("README.md").canonicalize().unwrap());
        let previous = vec![
            ProjectDocumentSnapshot {
                canonical_uri: readme_uri.clone(),
                content_hash: Artifact::compute_content_hash("# Project README\nhello"),
            },
            ProjectDocumentSnapshot {
                canonical_uri: "file:///missing.md".to_string(),
                content_hash: "old".to_string(),
            },
        ];

        let plan = LocalProjectDocumentSyncEngine::new(&docs)
            .scan(&previous)
            .unwrap();

        assert_eq!(plan.documents.len(), 2);
        assert_eq!(plan.missing.len(), 1);
        assert_eq!(plan.missing[0].sync_state, DocumentSyncState::Missing);
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].sync_state, DocumentSyncState::Deleted);
        let readme = plan
            .documents
            .iter()
            .find(|document| document.canonical_uri == readme_uri)
            .unwrap();
        assert_eq!(readme.title, "Project README");
        assert_eq!(readme.sync_state, DocumentSyncState::Clean);
        let guide = plan
            .documents
            .iter()
            .find(|document| document.title == "Guide")
            .unwrap();
        assert_eq!(guide.sync_state, DocumentSyncState::Changed);
        assert!(
            !plan
                .documents
                .iter()
                .any(|document| document.canonical_uri.ends_with("skip.json"))
        );
    }

    #[test]
    fn local_project_document_sync_respects_excluded_directory_names() {
        let tempdir = tempfile::tempdir().unwrap();
        let docs = tempdir.path().join("docs");
        std::fs::create_dir_all(docs.join("reports")).unwrap();
        std::fs::write(docs.join("README.md"), "# Project README\nhello").unwrap();
        std::fs::write(
            docs.join("reports").join("markdown-docs-sync-plan.md"),
            "# Generated Report\nignore me",
        )
        .unwrap();

        let plan = LocalProjectDocumentSyncEngine::new(&docs)
            .with_excluded_dir_names(["reports"])
            .scan(&[])
            .unwrap();

        assert_eq!(plan.documents.len(), 1);
        assert_eq!(plan.documents[0].title, "Project README");
    }

    #[test]
    fn project_document_conflict_classifier_covers_clean_changed_deleted_and_conflicted() {
        let clean = classify_project_document_conflict(ProjectDocumentConflictInput {
            canonical_uri: "file:///clean.md".to_string(),
            base_content_hash: Some("same".to_string()),
            indexed_content_hash: Some("same".to_string()),
            local_content_hash: Some("same".to_string()),
        });
        assert_eq!(clean.sync_state, DocumentSyncState::Clean);
        assert_eq!(clean.conflict_state, DocumentConflictState::None);

        let changed = classify_project_document_conflict(ProjectDocumentConflictInput {
            canonical_uri: "file:///changed.md".to_string(),
            base_content_hash: Some("base".to_string()),
            indexed_content_hash: Some("base".to_string()),
            local_content_hash: Some("local".to_string()),
        });
        assert_eq!(changed.sync_state, DocumentSyncState::Changed);
        assert_eq!(changed.conflict_state, DocumentConflictState::LocalChanged);

        let deleted = classify_project_document_conflict(ProjectDocumentConflictInput {
            canonical_uri: "file:///deleted.md".to_string(),
            base_content_hash: Some("base".to_string()),
            indexed_content_hash: Some("base".to_string()),
            local_content_hash: None,
        });
        assert_eq!(deleted.sync_state, DocumentSyncState::Deleted);
        assert_eq!(deleted.conflict_state, DocumentConflictState::None);

        let conflicted = classify_project_document_conflict(ProjectDocumentConflictInput {
            canonical_uri: "file:///conflict.md".to_string(),
            base_content_hash: Some("base".to_string()),
            indexed_content_hash: Some("remote".to_string()),
            local_content_hash: Some("local".to_string()),
        });
        assert_eq!(conflicted.sync_state, DocumentSyncState::Conflicted);
        assert_eq!(
            conflicted.conflict_state,
            DocumentConflictState::BothChanged
        );
        assert!(conflicted.reason.is_some());
    }
}
