use crate::{
    MemoryLayer, ScopeId, Sensitivity, Visibility, memory::MemoryKind, memory::MemoryState,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecordNativeKind {
    AgentContext,
    ProjectDocument,
    Memory,
}

impl MemoryRecordNativeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentContext => "agent_context",
            Self::ProjectDocument => "project_document",
            Self::Memory => "memory",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecordType {
    Fact,
    Preference,
    Decision,
    Procedure,
    Constraint,
    Risk,
    Summary,
    Insight,
    TaskState,
    Issue,
    CodeContext,
    MeetingNote,
    Hypothesis,
}

impl MemoryRecordType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Preference => "preference",
            Self::Decision => "decision",
            Self::Procedure => "procedure",
            Self::Constraint => "constraint",
            Self::Risk => "risk",
            Self::Summary => "summary",
            Self::Insight => "insight",
            Self::TaskState => "task_state",
            Self::Issue => "issue",
            Self::CodeContext => "code_context",
            Self::MeetingNote => "meeting_note",
            Self::Hypothesis => "hypothesis",
        }
    }

    pub fn from_memory_kind(kind: MemoryKind) -> Self {
        match kind {
            MemoryKind::Fact => Self::Fact,
            MemoryKind::Preference => Self::Preference,
            MemoryKind::Decision => Self::Decision,
            MemoryKind::Procedure => Self::Procedure,
            MemoryKind::Constraint => Self::Constraint,
            MemoryKind::Risk => Self::Risk,
            MemoryKind::Summary => Self::Summary,
            MemoryKind::Insight => Self::Insight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecordSourceKind {
    Conversation,
    File,
    Api,
    Meeting,
    Manual,
    System,
    Legacy,
}

impl MemoryRecordSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Conversation => "conversation",
            Self::File => "file",
            Self::Api => "api",
            Self::Meeting => "meeting",
            Self::Manual => "manual",
            Self::System => "system",
            Self::Legacy => "legacy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecordConfidence {
    Explicit,
    Inferred,
    Unverified,
    Stale,
}

impl MemoryRecordConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Inferred => "inferred",
            Self::Unverified => "unverified",
            Self::Stale => "stale",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRecordStatus {
    Candidate,
    Active,
    NeedsReview,
    Archived,
    Deprecated,
    Forgotten,
    Deleted,
}

impl MemoryRecordStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Active => "active",
            Self::NeedsReview => "needs_review",
            Self::Archived => "archived",
            Self::Deprecated => "deprecated",
            Self::Forgotten => "forgotten",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_memory_state(state: MemoryState) -> Self {
        match state {
            MemoryState::Candidate => Self::Candidate,
            MemoryState::Active => Self::Active,
            MemoryState::Conflicted => Self::NeedsReview,
            MemoryState::Archived => Self::Archived,
            MemoryState::Deprecated => Self::Deprecated,
            MemoryState::Forgotten => Self::Forgotten,
            MemoryState::Deleted => Self::Deleted,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub record_id: String,
    pub native_id: String,
    pub native_kind: MemoryRecordNativeKind,
    pub layer: MemoryLayer,
    pub scope_id: ScopeId,
    pub owner_scope_id: Option<ScopeId>,
    pub record_type: MemoryRecordType,
    pub title: String,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub source_kind: MemoryRecordSourceKind,
    pub source_ref: Option<String>,
    pub confidence: MemoryRecordConfidence,
    pub status: MemoryRecordStatus,
    pub visibility: Visibility,
    pub sensitivity: Sensitivity,
    pub importance: f32,
    pub freshness: f32,
    pub stability: f32,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl MemoryRecord {
    pub fn new(
        native_kind: MemoryRecordNativeKind,
        native_id: impl Into<String>,
        layer: MemoryLayer,
        scope_id: ScopeId,
        record_type: MemoryRecordType,
        title: impl Into<String>,
    ) -> Self {
        let native_id = native_id.into();
        let now = OffsetDateTime::now_utc();
        Self {
            record_id: format!("{}:{}", native_kind.as_str(), native_id),
            native_id,
            native_kind,
            layer,
            scope_id,
            owner_scope_id: None,
            record_type,
            title: title.into(),
            content: None,
            summary: None,
            source_kind: MemoryRecordSourceKind::Manual,
            source_ref: None,
            confidence: MemoryRecordConfidence::Explicit,
            status: MemoryRecordStatus::Active,
            visibility: Visibility::Private,
            sensitivity: Sensitivity::Internal,
            importance: 0.5,
            freshness: 1.0,
            stability: 0.5,
            expires_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MemoryRecord, MemoryRecordNativeKind, MemoryRecordStatus, MemoryRecordType};
    use crate::{MemoryLayer, MemoryState, ScopeId};

    #[test]
    fn memory_record_builds_stable_record_id() {
        let record = MemoryRecord::new(
            MemoryRecordNativeKind::Memory,
            "mem_123",
            MemoryLayer::LongTerm,
            ScopeId::from_string("scp_v27"),
            MemoryRecordType::Decision,
            "Lifecycle decision",
        );

        assert_eq!(record.record_id, "memory:mem_123");
        assert_eq!(record.native_id, "mem_123");
        assert_eq!(record.layer, MemoryLayer::LongTerm);
    }

    #[test]
    fn record_status_maps_memory_conflicted_to_needs_review() {
        assert_eq!(
            MemoryRecordStatus::from_memory_state(MemoryState::Conflicted),
            MemoryRecordStatus::NeedsReview
        );
    }
}
