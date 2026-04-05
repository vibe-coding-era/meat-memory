use anyhow::{Context, Result};
use memory_domain::{Episode, Memory};
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
    pub memory_kind: String,
    pub title: String,
    pub status: String,
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
            memory_kind: memory_kind_to_str(memory.kind).to_string(),
            title: memory.title.clone(),
            status: memory.state.as_str().to_string(),
            visibility: visibility_to_str(memory.visibility).to_string(),
            sensitivity: sensitivity_to_str(memory.sensitivity).to_string(),
            created_at: format_timestamp(memory.created_at)?,
            updated_at: format_timestamp(memory.updated_at)?,
            source_refs: Vec::new(),
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
