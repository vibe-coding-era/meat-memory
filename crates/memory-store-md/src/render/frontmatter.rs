use anyhow::{Context, Result};
use memory_domain::{Episode, Memory, MemoryLayer, ProjectDocument, SourceId};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFrontmatterScores {
    pub confidence: f32,
    pub importance: f32,
    pub stability: f32,
    pub freshness: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFrontmatter {
    pub id: String,
    pub kind: String,
    pub tenant: String,
    pub scope: String,
    pub owner_scope: String,
    #[serde(default)]
    pub published_from_scope: Option<String>,
    pub memory_kind: String,
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub language_code: Option<String>,
    pub visibility: String,
    pub sensitivity: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub scores: MemoryFrontmatterScores,
    #[serde(default)]
    pub evidence_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpisodeFrontmatter {
    pub id: String,
    pub kind: String,
    pub tenant: String,
    pub scope: String,
    pub episode_kind: String,
    pub title: String,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    #[serde(default)]
    pub participants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDocumentFrontmatter {
    pub id: String,
    pub kind: String,
    pub tenant: String,
    pub source_id: String,
    pub scope: String,
    pub layer: String,
    pub title: String,
    pub canonical_uri: String,
    #[serde(default)]
    pub local_path: Option<String>,
    pub content_hash: String,
    pub sync_state: String,
    pub conflict_state: String,
    #[serde(default)]
    pub artifact_id: Option<String>,
    #[serde(default)]
    pub memory_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn render_frontmatter<T: Serialize>(value: &T) -> Result<String> {
    let yaml = serde_yaml::to_string(value).context("failed to serialize markdown frontmatter")?;
    Ok(format!("---\n{yaml}---\n"))
}

impl MemoryFrontmatter {
    pub fn from_memory(memory: &Memory, tenant: &str) -> Result<Self> {
        Ok(Self {
            id: memory.id.as_str().to_string(),
            kind: "memory".to_string(),
            tenant: tenant.to_string(),
            scope: memory.scope_id.as_str().to_string(),
            owner_scope: memory.owner_scope_id.as_str().to_string(),
            published_from_scope: memory
                .published_from_scope_id
                .as_ref()
                .map(|scope| scope.as_str().to_string()),
            memory_kind: memory_kind_to_str(memory.kind).to_string(),
            title: memory.title.clone(),
            status: memory.state.as_str().to_string(),
            language_code: memory.language_code.clone(),
            visibility: visibility_to_str(memory.visibility).to_string(),
            sensitivity: sensitivity_to_str(memory.sensitivity).to_string(),
            created_at: format_timestamp(memory.created_at)?,
            updated_at: format_timestamp(memory.updated_at)?,
            source_refs: memory.source_refs.clone(),
            evidence: Vec::new(),
            entities: Vec::new(),
            tags: Vec::new(),
            scores: MemoryFrontmatterScores {
                confidence: memory.scores.confidence,
                importance: memory.scores.importance,
                stability: memory.scores.stability,
                freshness: memory.scores.freshness,
            },
            evidence_count: memory.evidence_count,
        })
    }
}

impl EpisodeFrontmatter {
    pub fn from_episode(episode: &Episode, tenant: &str) -> Result<Self> {
        Ok(Self {
            id: episode.id.as_str().to_string(),
            kind: "episode".to_string(),
            tenant: tenant.to_string(),
            scope: episode.scope_id.as_str().to_string(),
            episode_kind: episode_kind_to_str(episode.kind).to_string(),
            title: episode.title.clone(),
            status: episode.state.as_str().to_string(),
            started_at: format_timestamp(episode.started_at)?,
            ended_at: episode
                .ended_at
                .map(format_timestamp)
                .transpose()
                .context("failed to format episode end timestamp")?,
            participants: episode.participants.clone(),
        })
    }
}

impl ProjectDocumentFrontmatter {
    pub fn from_project_document(
        document: &ProjectDocument,
        tenant: &str,
        source_id: &SourceId,
    ) -> Result<Self> {
        Ok(Self {
            id: document.id.as_str().to_string(),
            kind: "project_document".to_string(),
            tenant: tenant.to_string(),
            source_id: source_id.as_str().to_string(),
            scope: document.scope_id.as_str().to_string(),
            layer: MemoryLayer::MidTerm.as_str().to_string(),
            title: document.title.clone(),
            canonical_uri: document.canonical_uri.clone(),
            local_path: document.local_path.clone(),
            content_hash: document.content_hash.clone(),
            sync_state: document.sync_state.as_str().to_string(),
            conflict_state: document.conflict_state.as_str().to_string(),
            artifact_id: document
                .artifact_id
                .as_ref()
                .map(|artifact_id| artifact_id.as_str().to_string()),
            memory_id: document
                .memory_id
                .as_ref()
                .map(|memory_id| memory_id.as_str().to_string()),
            created_at: format_timestamp(document.created_at)?,
            updated_at: format_timestamp(document.updated_at)?,
        })
    }
}

fn format_timestamp(value: OffsetDateTime) -> Result<String> {
    value
        .format(&Rfc3339)
        .context("failed to format timestamp as RFC3339")
}

fn memory_kind_to_str(kind: memory_domain::MemoryKind) -> &'static str {
    match kind {
        memory_domain::MemoryKind::Fact => "fact",
        memory_domain::MemoryKind::Preference => "preference",
        memory_domain::MemoryKind::Decision => "decision",
        memory_domain::MemoryKind::Procedure => "procedure",
        memory_domain::MemoryKind::Constraint => "constraint",
        memory_domain::MemoryKind::Risk => "risk",
        memory_domain::MemoryKind::Summary => "summary",
        memory_domain::MemoryKind::Insight => "insight",
    }
}

fn visibility_to_str(visibility: memory_domain::Visibility) -> &'static str {
    match visibility {
        memory_domain::Visibility::Private => "private",
        memory_domain::Visibility::Project => "project",
        memory_domain::Visibility::Team => "team",
        memory_domain::Visibility::Organization => "organization",
    }
}

fn sensitivity_to_str(sensitivity: memory_domain::Sensitivity) -> &'static str {
    match sensitivity {
        memory_domain::Sensitivity::Public => "public",
        memory_domain::Sensitivity::Internal => "internal",
        memory_domain::Sensitivity::Private => "private",
        memory_domain::Sensitivity::Restricted => "restricted",
    }
}

fn episode_kind_to_str(kind: memory_domain::EpisodeKind) -> &'static str {
    match kind {
        memory_domain::EpisodeKind::ChatSession => "chat_session",
        memory_domain::EpisodeKind::CodingTask => "coding_task",
        memory_domain::EpisodeKind::Meeting => "meeting",
        memory_domain::EpisodeKind::ResearchRun => "research_run",
        memory_domain::EpisodeKind::AutomationRun => "automation_run",
        memory_domain::EpisodeKind::Incident => "incident",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EpisodeFrontmatter, MemoryFrontmatter, MemoryFrontmatterScores, ProjectDocumentFrontmatter,
        episode_kind_to_str, format_timestamp, memory_kind_to_str, render_frontmatter,
        sensitivity_to_str, visibility_to_str,
    };
    use memory_domain::{
        DocumentConflictState, DocumentSyncState, Episode, EpisodeKind, Memory, MemoryKind,
        MemoryScores, ProjectDocument, ScopeId, Sensitivity, SourceId, Visibility,
    };
    use time::macros::datetime;

    fn sample_memory(kind: MemoryKind) -> Memory {
        let mut memory = Memory::new(
            ScopeId::from_string("scp_frontmatter"),
            kind,
            "记忆标题",
            "记忆正文",
        )
        .expect("memory should build");
        memory.id = memory_domain::MemoryId::from_string("mem_frontmatter");
        memory.source_refs = vec!["agent-context://ctx_frontmatter".to_string()];
        memory.created_at = datetime!(2025-01-02 03:04:05 UTC);
        memory.updated_at = datetime!(2025-01-03 04:05:06 UTC);
        memory.scores = MemoryScores {
            confidence: 0.8,
            importance: 0.7,
            stability: 0.6,
            freshness: 0.5,
        };
        memory.evidence_count = 2;
        memory
    }

    fn sample_episode(kind: EpisodeKind) -> Episode {
        let mut episode = Episode::new(ScopeId::from_string("scp_frontmatter"), kind, "迭代回顾")
            .expect("episode should build");
        episode.id = memory_domain::EpisodeId::from_string("epi_frontmatter");
        episode.started_at = datetime!(2025-01-02 03:04:05 UTC);
        episode
    }

    fn sample_project_document() -> ProjectDocument {
        let mut document = ProjectDocument::new(
            SourceId::from_string("src_frontmatter"),
            ScopeId::from_string("scp_frontmatter"),
            "file:///tmp/README.md",
            "README",
            "sha256:frontmatter",
        )
        .expect("project document should build");
        document.id = memory_domain::ProjectDocumentId::from_string("doc_frontmatter");
        document.local_path = Some("/tmp/README.md".to_string());
        document.sync_state = DocumentSyncState::Changed;
        document.conflict_state = DocumentConflictState::LocalChanged;
        document.created_at = datetime!(2025-01-02 03:04:05 UTC);
        document.updated_at = datetime!(2025-01-03 04:05:06 UTC);
        document
    }

    #[test]
    fn render_frontmatter_wraps_serialized_yaml() {
        let rendered = render_frontmatter(&MemoryFrontmatterScores {
            confidence: 0.9,
            importance: 0.8,
            stability: 0.7,
            freshness: 0.6,
        })
        .expect("frontmatter should render");

        assert!(rendered.starts_with("---\n"));
        assert!(rendered.ends_with("---\n"));
        assert!(rendered.contains("confidence: 0.9"));
    }

    #[test]
    fn memory_frontmatter_from_memory_maps_all_fields() {
        let memory_kinds = [
            (MemoryKind::Fact, "fact"),
            (MemoryKind::Preference, "preference"),
            (MemoryKind::Decision, "decision"),
            (MemoryKind::Procedure, "procedure"),
            (MemoryKind::Constraint, "constraint"),
            (MemoryKind::Risk, "risk"),
            (MemoryKind::Summary, "summary"),
            (MemoryKind::Insight, "insight"),
        ];
        let visibilities = [
            (Visibility::Private, "private"),
            (Visibility::Project, "project"),
            (Visibility::Team, "team"),
            (Visibility::Organization, "organization"),
        ];
        let sensitivities = [
            (Sensitivity::Public, "public"),
            (Sensitivity::Internal, "internal"),
            (Sensitivity::Private, "private"),
            (Sensitivity::Restricted, "restricted"),
        ];

        for (index, (kind, kind_label)) in memory_kinds.into_iter().enumerate() {
            let mut memory = sample_memory(kind);
            let (visibility, visibility_label) = visibilities[index % visibilities.len()];
            let (sensitivity, sensitivity_label) = sensitivities[index % sensitivities.len()];
            memory.visibility = visibility;
            memory.sensitivity = sensitivity;

            let frontmatter =
                MemoryFrontmatter::from_memory(&memory, "tenant-a").expect("memory should map");

            assert_eq!(frontmatter.id, "mem_frontmatter");
            assert_eq!(frontmatter.kind, "memory");
            assert_eq!(frontmatter.tenant, "tenant-a");
            assert_eq!(frontmatter.scope, "scp_frontmatter");
            assert_eq!(frontmatter.owner_scope, "scp_frontmatter");
            assert_eq!(frontmatter.published_from_scope, None);
            assert_eq!(frontmatter.memory_kind, kind_label);
            assert_eq!(frontmatter.title, "记忆标题");
            assert_eq!(frontmatter.status, "candidate");
            assert_eq!(frontmatter.language_code, None);
            assert_eq!(frontmatter.visibility, visibility_label);
            assert_eq!(frontmatter.sensitivity, sensitivity_label);
            assert_eq!(frontmatter.created_at, "2025-01-02T03:04:05Z");
            assert_eq!(frontmatter.updated_at, "2025-01-03T04:05:06Z");
            assert_eq!(
                frontmatter.source_refs,
                vec!["agent-context://ctx_frontmatter"]
            );
            assert!(frontmatter.evidence.is_empty());
            assert!(frontmatter.entities.is_empty());
            assert!(frontmatter.tags.is_empty());
            assert_eq!(frontmatter.scores.confidence, 0.8);
            assert_eq!(frontmatter.evidence_count, 2);
        }
    }

    #[test]
    fn episode_frontmatter_from_episode_maps_all_fields() {
        let episode_kinds = [
            (EpisodeKind::ChatSession, "chat_session"),
            (EpisodeKind::CodingTask, "coding_task"),
            (EpisodeKind::Meeting, "meeting"),
            (EpisodeKind::ResearchRun, "research_run"),
            (EpisodeKind::AutomationRun, "automation_run"),
            (EpisodeKind::Incident, "incident"),
        ];

        for (index, (kind, expected_kind)) in episode_kinds.into_iter().enumerate() {
            let mut episode = sample_episode(kind);
            episode.participants = vec!["rou".to_string(), "agent".to_string()];
            if index % 2 == 0 {
                episode.ended_at = Some(datetime!(2025-01-02 05:06:07 UTC));
            }

            let frontmatter =
                EpisodeFrontmatter::from_episode(&episode, "tenant-a").expect("episode maps");

            assert_eq!(frontmatter.id, "epi_frontmatter");
            assert_eq!(frontmatter.kind, "episode");
            assert_eq!(frontmatter.tenant, "tenant-a");
            assert_eq!(frontmatter.scope, "scp_frontmatter");
            assert_eq!(frontmatter.episode_kind, expected_kind);
            assert_eq!(frontmatter.title, "迭代回顾");
            assert_eq!(frontmatter.status, "open");
            assert_eq!(frontmatter.started_at, "2025-01-02T03:04:05Z");
            assert_eq!(frontmatter.participants.len(), 2);
            if index % 2 == 0 {
                assert_eq!(
                    frontmatter.ended_at.as_deref(),
                    Some("2025-01-02T05:06:07Z")
                );
            } else {
                assert_eq!(frontmatter.ended_at, None);
            }
        }
    }

    #[test]
    fn project_document_frontmatter_from_document_maps_all_fields() {
        let document = sample_project_document();
        let frontmatter = ProjectDocumentFrontmatter::from_project_document(
            &document,
            "tenant-a",
            &SourceId::from_string("src_frontmatter"),
        )
        .expect("project document should map");

        assert_eq!(frontmatter.id, "doc_frontmatter");
        assert_eq!(frontmatter.kind, "project_document");
        assert_eq!(frontmatter.tenant, "tenant-a");
        assert_eq!(frontmatter.source_id, "src_frontmatter");
        assert_eq!(frontmatter.scope, "scp_frontmatter");
        assert_eq!(frontmatter.layer, "mid_term");
        assert_eq!(frontmatter.title, "README");
        assert_eq!(frontmatter.canonical_uri, "file:///tmp/README.md");
        assert_eq!(frontmatter.local_path.as_deref(), Some("/tmp/README.md"));
        assert_eq!(frontmatter.content_hash, "sha256:frontmatter");
        assert_eq!(frontmatter.sync_state, "changed");
        assert_eq!(frontmatter.conflict_state, "local_changed");
        assert_eq!(frontmatter.created_at, "2025-01-02T03:04:05Z");
        assert_eq!(frontmatter.updated_at, "2025-01-03T04:05:06Z");
    }

    #[test]
    fn helper_label_functions_cover_all_variants() {
        let memory_kind_cases = [
            (MemoryKind::Fact, "fact"),
            (MemoryKind::Preference, "preference"),
            (MemoryKind::Decision, "decision"),
            (MemoryKind::Procedure, "procedure"),
            (MemoryKind::Constraint, "constraint"),
            (MemoryKind::Risk, "risk"),
            (MemoryKind::Summary, "summary"),
            (MemoryKind::Insight, "insight"),
        ];
        for (kind, expected) in memory_kind_cases {
            assert_eq!(memory_kind_to_str(kind), expected);
        }

        let visibility_cases = [
            (Visibility::Private, "private"),
            (Visibility::Project, "project"),
            (Visibility::Team, "team"),
            (Visibility::Organization, "organization"),
        ];
        for (visibility, expected) in visibility_cases {
            assert_eq!(visibility_to_str(visibility), expected);
        }

        let sensitivity_cases = [
            (Sensitivity::Public, "public"),
            (Sensitivity::Internal, "internal"),
            (Sensitivity::Private, "private"),
            (Sensitivity::Restricted, "restricted"),
        ];
        for (sensitivity, expected) in sensitivity_cases {
            assert_eq!(sensitivity_to_str(sensitivity), expected);
        }

        let episode_kind_cases = [
            (EpisodeKind::ChatSession, "chat_session"),
            (EpisodeKind::CodingTask, "coding_task"),
            (EpisodeKind::Meeting, "meeting"),
            (EpisodeKind::ResearchRun, "research_run"),
            (EpisodeKind::AutomationRun, "automation_run"),
            (EpisodeKind::Incident, "incident"),
        ];
        for (kind, expected) in episode_kind_cases {
            assert_eq!(episode_kind_to_str(kind), expected);
        }
    }

    #[test]
    fn format_timestamp_emits_rfc3339() {
        let formatted =
            format_timestamp(datetime!(2025-01-02 03:04:05 UTC)).expect("timestamp should format");
        assert_eq!(formatted, "2025-01-02T03:04:05Z");
    }
}
