use crate::{DomainError, ProjectIdentityBindingId, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectBindingKind {
    RepoRoot,
    RemoteUrl,
    SourceRefPrefix,
    Alias,
}

impl ProjectBindingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RepoRoot => "repo_root",
            Self::RemoteUrl => "remote_url",
            Self::SourceRefPrefix => "source_ref_prefix",
            Self::Alias => "alias",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingConfirmedBy {
    User,
    System,
}

impl BindingConfirmedBy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectIdentityBinding {
    pub id: ProjectIdentityBindingId,
    pub owner_scope_id: ScopeId,
    pub binding_kind: ProjectBindingKind,
    pub binding_value: String,
    pub canonical_project_key: String,
    pub confirmed_by: BindingConfirmedBy,
    pub confidence: f32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl ProjectIdentityBinding {
    pub fn new(
        owner_scope_id: ScopeId,
        binding_kind: ProjectBindingKind,
        binding_value: impl Into<String>,
        canonical_project_key: impl Into<String>,
        confirmed_by: BindingConfirmedBy,
    ) -> Result<Self, DomainError> {
        let now = OffsetDateTime::now_utc();
        Ok(Self {
            id: ProjectIdentityBindingId::new(),
            owner_scope_id,
            binding_kind,
            binding_value: non_empty(
                binding_value.into(),
                "project_identity_binding.binding_value",
            )?,
            canonical_project_key: non_empty(
                canonical_project_key.into(),
                "project_identity_binding.canonical_project_key",
            )?,
            confirmed_by,
            confidence: 1.0,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_confidence(mut self, confidence: f32) -> Result<Self, DomainError> {
        self.confidence = validate_confidence(confidence, "project_identity_binding.confidence")?;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(self)
    }

    pub fn matches(&self, binding_kind: ProjectBindingKind, binding_value: &str) -> bool {
        self.binding_kind == binding_kind && self.binding_value == binding_value
    }
}

fn non_empty(value: String, field: &'static str) -> Result<String, DomainError> {
    if value.trim().is_empty() {
        return Err(DomainError::EmptyField { field });
    }
    Ok(value)
}

fn validate_confidence(value: f32, field: &'static str) -> Result<f32, DomainError> {
    if !(0.0..=1.0).contains(&value) {
        return Err(DomainError::InvalidScore {
            field,
            value: value.to_string(),
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{BindingConfirmedBy, ProjectBindingKind, ProjectIdentityBinding};
    use crate::ScopeId;

    #[test]
    fn binding_rejects_empty_identity_fields() {
        assert!(
            ProjectIdentityBinding::new(
                ScopeId::from_string("scp_v28"),
                ProjectBindingKind::RepoRoot,
                " ",
                "proj_meat_memory",
                BindingConfirmedBy::User,
            )
            .is_err()
        );
        assert!(
            ProjectIdentityBinding::new(
                ScopeId::from_string("scp_v28"),
                ProjectBindingKind::RepoRoot,
                "/Users/Rou/dev_projects/meat-memory",
                " ",
                BindingConfirmedBy::User,
            )
            .is_err()
        );
    }

    #[test]
    fn binding_tracks_kind_value_and_confidence() {
        let binding = ProjectIdentityBinding::new(
            ScopeId::from_string("scp_v28"),
            ProjectBindingKind::RepoRoot,
            "/Users/Rou/dev_projects/meat-memory",
            "proj_meat_memory",
            BindingConfirmedBy::User,
        )
        .unwrap()
        .with_confidence(0.95)
        .unwrap();

        assert!(binding.id.as_str().starts_with("pib_"));
        assert_eq!(binding.binding_kind.as_str(), "repo_root");
        assert!(binding.matches(
            ProjectBindingKind::RepoRoot,
            "/Users/Rou/dev_projects/meat-memory"
        ));
        assert_eq!(binding.canonical_project_key, "proj_meat_memory");
        assert_eq!(binding.confirmed_by.as_str(), "user");
        assert_eq!(binding.confidence, 0.95);
    }

    #[test]
    fn binding_confidence_must_stay_within_zero_and_one() {
        assert!(
            ProjectIdentityBinding::new(
                ScopeId::from_string("scp_v28"),
                ProjectBindingKind::Alias,
                "meat-memory",
                "proj_meat_memory",
                BindingConfirmedBy::System,
            )
            .unwrap()
            .with_confidence(1.2)
            .is_err()
        );
    }

    #[test]
    fn binding_kind_and_confirmed_by_render_expected_strings() {
        assert_eq!(ProjectBindingKind::RepoRoot.as_str(), "repo_root");
        assert_eq!(ProjectBindingKind::RemoteUrl.as_str(), "remote_url");
        assert_eq!(
            ProjectBindingKind::SourceRefPrefix.as_str(),
            "source_ref_prefix"
        );
        assert_eq!(ProjectBindingKind::Alias.as_str(), "alias");
        assert_eq!(BindingConfirmedBy::User.as_str(), "user");
        assert_eq!(BindingConfirmedBy::System.as_str(), "system");
    }

    #[test]
    fn binding_matches_can_fail_for_different_kind_or_value() {
        let binding = ProjectIdentityBinding::new(
            ScopeId::from_string("scp_v28"),
            ProjectBindingKind::RepoRoot,
            "/Users/Rou/dev_projects/meat-memory",
            "proj_meat_memory",
            BindingConfirmedBy::User,
        )
        .unwrap();

        assert!(!binding.matches(
            ProjectBindingKind::Alias,
            "/Users/Rou/dev_projects/meat-memory"
        ));
        assert!(!binding.matches(ProjectBindingKind::RepoRoot, "/tmp/other"));
    }
}
