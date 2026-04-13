use crate::{
    AccessKeyId, AgentContextId, ArtifactId, DomainError, MemoryId, ProjectDocumentId, ScopeId,
    SourceId,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLayer {
    ShortTerm,
    MidTerm,
    LongTerm,
}

impl MemoryLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShortTerm => "short_term",
            Self::MidTerm => "mid_term",
            Self::LongTerm => "long_term",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "short_term" => Ok(Self::ShortTerm),
            "mid_term" => Ok(Self::MidTerm),
            "long_term" => Ok(Self::LongTerm),
            _ => Err(DomainError::InvalidField {
                field: "memory_layer",
                reason: "unsupported memory layer",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceSyncMode {
    ReadOnly,
    IndexOnly,
    TwoWay,
}

impl SourceSyncMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::IndexOnly => "index_only",
            Self::TwoWay => "two_way",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "read_only" => Ok(Self::ReadOnly),
            "index_only" => Ok(Self::IndexOnly),
            "two_way" => Ok(Self::TwoWay),
            _ => Err(DomainError::InvalidField {
                field: "memory_source.sync_mode",
                reason: "unsupported sync mode",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceStatus {
    Active,
    Disabled,
    Archived,
}

impl SourceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Archived => "archived",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainError::InvalidField {
                field: "memory_source.status",
                reason: "unsupported source status",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySource {
    pub id: SourceId,
    pub source_kind: String,
    pub display_name: String,
    pub owner_principal_id: String,
    pub owner_scope_id: ScopeId,
    pub source_uri: Option<String>,
    pub sync_mode: SourceSyncMode,
    pub local_root: Option<String>,
    pub status: SourceStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MemorySource {
    pub fn new(
        source_kind: impl Into<String>,
        display_name: impl Into<String>,
        owner_principal_id: impl Into<String>,
        owner_scope_id: ScopeId,
    ) -> Result<Self, DomainError> {
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: SourceId::new(),
            source_kind: non_empty(source_kind.into(), "memory_source.source_kind")?,
            display_name: non_empty(display_name.into(), "memory_source.display_name")?,
            owner_principal_id: non_empty(
                owner_principal_id.into(),
                "memory_source.owner_principal_id",
            )?,
            owner_scope_id,
            source_uri: None,
            sync_mode: SourceSyncMode::ReadOnly,
            local_root: None,
            status: SourceStatus::Active,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_source_uri(mut self, source_uri: impl Into<String>) -> Result<Self, DomainError> {
        self.source_uri = Some(non_empty(source_uri.into(), "memory_source.source_uri")?);
        self.updated_at = OffsetDateTime::now_utc();
        Ok(self)
    }

    pub fn with_local_root(mut self, local_root: impl Into<String>) -> Result<Self, DomainError> {
        self.local_root = Some(non_empty(local_root.into(), "memory_source.local_root")?);
        self.updated_at = OffsetDateTime::now_utc();
        Ok(self)
    }

    pub fn with_sync_mode(mut self, sync_mode: SourceSyncMode) -> Self {
        self.sync_mode = sync_mode;
        self.updated_at = OffsetDateTime::now_utc();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContext {
    pub id: AgentContextId,
    pub source_id: Option<SourceId>,
    pub key_id: Option<AccessKeyId>,
    pub scope_id: ScopeId,
    pub session_id: String,
    pub task_id: Option<String>,
    pub layer: MemoryLayer,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl AgentContext {
    pub fn new(
        scope_id: ScopeId,
        session_id: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: AgentContextId::new(),
            source_id: None,
            key_id: None,
            scope_id,
            session_id: non_empty(session_id.into(), "agent_context.session_id")?,
            task_id: None,
            layer: MemoryLayer::ShortTerm,
            title: non_empty(title.into(), "agent_context.title")?,
            body: non_empty(body.into(), "agent_context.body")?,
            labels: Vec::new(),
            expires_at: None,
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentSyncState {
    Clean,
    Changed,
    Deleted,
    Missing,
    Conflicted,
}

impl DocumentSyncState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Changed => "changed",
            Self::Deleted => "deleted",
            Self::Missing => "missing",
            Self::Conflicted => "conflicted",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "clean" => Ok(Self::Clean),
            "changed" => Ok(Self::Changed),
            "deleted" => Ok(Self::Deleted),
            "missing" => Ok(Self::Missing),
            "conflicted" => Ok(Self::Conflicted),
            _ => Err(DomainError::InvalidField {
                field: "project_document.sync_state",
                reason: "unsupported document sync state",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentConflictState {
    None,
    LocalChanged,
    RemoteChanged,
    BothChanged,
}

impl DocumentConflictState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LocalChanged => "local_changed",
            Self::RemoteChanged => "remote_changed",
            Self::BothChanged => "both_changed",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "none" => Ok(Self::None),
            "local_changed" => Ok(Self::LocalChanged),
            "remote_changed" => Ok(Self::RemoteChanged),
            "both_changed" => Ok(Self::BothChanged),
            _ => Err(DomainError::InvalidField {
                field: "project_document.conflict_state",
                reason: "unsupported document conflict state",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub id: ProjectDocumentId,
    pub source_id: SourceId,
    pub scope_id: ScopeId,
    pub local_path: Option<String>,
    pub canonical_uri: String,
    pub title: String,
    pub content_hash: String,
    pub last_seen_mtime: Option<OffsetDateTime>,
    pub sync_state: DocumentSyncState,
    pub conflict_state: DocumentConflictState,
    pub artifact_id: Option<ArtifactId>,
    pub memory_id: Option<MemoryId>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl ProjectDocument {
    pub fn new(
        source_id: SourceId,
        scope_id: ScopeId,
        canonical_uri: impl Into<String>,
        title: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: ProjectDocumentId::new(),
            source_id,
            scope_id,
            local_path: None,
            canonical_uri: non_empty(canonical_uri.into(), "project_document.canonical_uri")?,
            title: non_empty(title.into(), "project_document.title")?,
            content_hash: non_empty(content_hash.into(), "project_document.content_hash")?,
            last_seen_mtime: None,
            sync_state: DocumentSyncState::Clean,
            conflict_state: DocumentConflictState::None,
            artifact_id: None,
            memory_id: None,
            created_at: now,
            updated_at: now,
        })
    }
}

fn non_empty(value: String, field: &'static str) -> Result<String, DomainError> {
    if value.trim().is_empty() {
        return Err(DomainError::EmptyField { field });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        AgentContext, DocumentConflictState, DocumentSyncState, MemoryLayer, MemorySource,
        ProjectDocument, SourceSyncMode,
    };
    use crate::{ScopeId, SourceId};

    #[test]
    fn memory_layer_roundtrips_storage_values() {
        assert_eq!(MemoryLayer::ShortTerm.as_str(), "short_term");
        assert_eq!(
            MemoryLayer::parse("mid_term").unwrap(),
            MemoryLayer::MidTerm
        );
        assert!(MemoryLayer::parse("forever-ish").is_err());
    }

    #[test]
    fn memory_source_defaults_to_read_only_active_source() {
        let source = MemorySource::new(
            "cli",
            "codex-local",
            "rou",
            ScopeId::from_string("scp_meat_memory_v1"),
        )
        .unwrap()
        .with_source_uri("agent://codex/local")
        .unwrap()
        .with_local_root("/Users/Rou/dev_projects/meat-memory")
        .unwrap()
        .with_sync_mode(SourceSyncMode::IndexOnly);

        assert_eq!(source.sync_mode, SourceSyncMode::IndexOnly);
        assert_eq!(source.source_uri.as_deref(), Some("agent://codex/local"));
        assert_eq!(
            source.local_root.as_deref(),
            Some("/Users/Rou/dev_projects/meat-memory")
        );
        assert_eq!(source.status.as_str(), "active");
    }

    #[test]
    fn agent_context_is_short_term_by_default() {
        let context = AgentContext::new(
            ScopeId::from_string("scp_meat_memory_v1"),
            "session-1",
            "Current task",
            "Design V2.4",
        )
        .unwrap();

        assert_eq!(context.layer, MemoryLayer::ShortTerm);
        assert_eq!(context.session_id, "session-1");
    }

    #[test]
    fn project_document_tracks_sync_and_conflict_state() {
        let document = ProjectDocument::new(
            SourceId::from_string("src_docs"),
            ScopeId::from_string("scp_meat_memory_v1"),
            "file:///repo/docs/README.md",
            "README",
            "sha256:abc",
        )
        .unwrap();

        assert_eq!(document.sync_state, DocumentSyncState::Clean);
        assert_eq!(document.conflict_state, DocumentConflictState::None);
        assert_eq!(DocumentSyncState::Conflicted.as_str(), "conflicted");
        assert_eq!(DocumentConflictState::BothChanged.as_str(), "both_changed");
    }

    #[test]
    fn rejects_empty_identity_fields() {
        assert!(MemorySource::new("cli", " ", "rou", ScopeId::from_string("scp")).is_err());
        assert!(AgentContext::new(ScopeId::from_string("scp"), " ", "title", "body").is_err());
    }
}
