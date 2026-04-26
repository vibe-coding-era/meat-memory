use crate::{DistillationProfileId, DistillationRunId, DomainError, MemoryKind, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistillationProfileLevel {
    UserGlobal,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistillationProfileStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistillationProfile {
    pub id: DistillationProfileId,
    pub scope_id: Option<ScopeId>,
    pub profile_level: DistillationProfileLevel,
    pub status: DistillationProfileStatus,
    pub name: String,
    pub prompt_text: String,
    pub focus_topics: Vec<String>,
    pub prefer_memory_kinds: Vec<MemoryKind>,
    pub created_by: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl DistillationProfile {
    pub fn new_global(
        name: impl Into<String>,
        prompt_text: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Result<Self, DomainError> {
        Self::new(
            None,
            DistillationProfileLevel::UserGlobal,
            name,
            prompt_text,
            created_by,
        )
    }

    pub fn new_project(
        scope_id: ScopeId,
        name: impl Into<String>,
        prompt_text: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Result<Self, DomainError> {
        Self::new(
            Some(scope_id),
            DistillationProfileLevel::Project,
            name,
            prompt_text,
            created_by,
        )
    }

    pub fn add_focus_topic(mut self, topic: impl Into<String>) -> Result<Self, DomainError> {
        let topic = non_empty(topic.into(), "distillation_profile.focus_topic")?;
        if !self.focus_topics.contains(&topic) {
            self.focus_topics.push(topic);
        }
        self.updated_at = OffsetDateTime::now_utc();
        Ok(self)
    }

    pub fn prefer_memory_kind(mut self, memory_kind: MemoryKind) -> Self {
        if !self.prefer_memory_kinds.contains(&memory_kind) {
            self.prefer_memory_kinds.push(memory_kind);
        }
        self.updated_at = OffsetDateTime::now_utc();
        self
    }

    pub fn archive(&mut self) {
        self.status = DistillationProfileStatus::Archived;
        self.updated_at = OffsetDateTime::now_utc();
    }

    pub fn activate(&mut self) {
        self.status = DistillationProfileStatus::Active;
        self.updated_at = OffsetDateTime::now_utc();
    }

    fn new(
        scope_id: Option<ScopeId>,
        profile_level: DistillationProfileLevel,
        name: impl Into<String>,
        prompt_text: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: DistillationProfileId::new(),
            scope_id,
            profile_level,
            status: DistillationProfileStatus::Active,
            name: non_empty(name.into(), "distillation_profile.name")?,
            prompt_text: non_empty(prompt_text.into(), "distillation_profile.prompt_text")?,
            focus_topics: Vec::new(),
            prefer_memory_kinds: Vec::new(),
            created_by: non_empty(created_by.into(), "distillation_profile.created_by")?,
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistillationRun {
    pub id: DistillationRunId,
    pub profile_id: Option<DistillationProfileId>,
    pub scope_id: ScopeId,
    pub input_hash: String,
    pub preview: bool,
    pub created_at: OffsetDateTime,
}

impl DistillationRun {
    pub fn new(
        profile_id: Option<DistillationProfileId>,
        scope_id: ScopeId,
        input_hash: impl Into<String>,
        preview: bool,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: DistillationRunId::new(),
            profile_id,
            scope_id,
            input_hash: non_empty(input_hash.into(), "distillation_run.input_hash")?,
            preview,
            created_at: OffsetDateTime::now_utc(),
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
        DistillationProfile, DistillationProfileLevel, DistillationProfileStatus, DistillationRun,
    };
    use crate::{MemoryKind, ScopeId};

    #[test]
    fn global_and_project_profiles_track_scope_rules() {
        let global = DistillationProfile::new_global(
            "Default profile",
            "Keep decisions and constraints",
            "user",
        )
        .unwrap();
        let project = DistillationProfile::new_project(
            ScopeId::from_string("scp_v28"),
            "Project profile",
            "Focus on V2.8 governance",
            "user",
        )
        .unwrap()
        .add_focus_topic("proposal")
        .unwrap()
        .prefer_memory_kind(MemoryKind::Decision);

        assert!(global.id.as_str().starts_with("dpf_"));
        assert_eq!(global.profile_level, DistillationProfileLevel::UserGlobal);
        assert!(global.scope_id.is_none());
        assert_eq!(project.profile_level, DistillationProfileLevel::Project);
        assert_eq!(project.scope_id.unwrap().as_str(), "scp_v28");
        assert_eq!(project.focus_topics, vec!["proposal".to_string()]);
        assert_eq!(project.prefer_memory_kinds, vec![MemoryKind::Decision]);
    }

    #[test]
    fn profile_archive_and_activate_roundtrip() {
        let mut profile = DistillationProfile::new_global(
            "Default profile",
            "Keep decisions and constraints",
            "user",
        )
        .unwrap();

        profile.archive();
        assert_eq!(profile.status, DistillationProfileStatus::Archived);
        profile.activate();
        assert_eq!(profile.status, DistillationProfileStatus::Active);
    }

    #[test]
    fn distillation_run_requires_non_empty_input_hash() {
        assert!(DistillationRun::new(None, ScopeId::from_string("scp_v28"), " ", true).is_err());

        let run = DistillationRun::new(None, ScopeId::from_string("scp_v28"), "sha256:abc", true)
            .unwrap();
        assert!(run.id.as_str().starts_with("drn_"));
        assert!(run.preview);
    }

    #[test]
    fn profile_validates_required_fields() {
        assert!(DistillationProfile::new_global(" ", "prompt", "user").is_err());
        assert!(DistillationProfile::new_global("name", " ", "user").is_err());
        assert!(DistillationProfile::new_global("name", "prompt", " ").is_err());
        assert!(
            DistillationProfile::new_project(
                ScopeId::from_string("scp_v28"),
                "name",
                "prompt",
                "user",
            )
            .unwrap()
            .add_focus_topic(" ")
            .is_err()
        );
    }

    #[test]
    fn profile_deduplicates_topics_and_memory_kinds() {
        let profile = DistillationProfile::new_project(
            ScopeId::from_string("scp_v28"),
            "Project profile",
            "Focus on V2.8 governance",
            "user",
        )
        .unwrap()
        .add_focus_topic("proposal")
        .unwrap()
        .add_focus_topic("proposal")
        .unwrap()
        .prefer_memory_kind(MemoryKind::Decision)
        .prefer_memory_kind(MemoryKind::Decision);

        assert_eq!(profile.focus_topics, vec!["proposal".to_string()]);
        assert_eq!(profile.prefer_memory_kinds, vec![MemoryKind::Decision]);
    }
}
