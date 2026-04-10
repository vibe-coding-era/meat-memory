use crate::{DomainError, MemoryId, ScopeId, Sensitivity, Visibility};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Fact,
    Preference,
    Decision,
    Procedure,
    Constraint,
    Risk,
    Summary,
    Insight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryState {
    Candidate,
    Active,
    Deprecated,
    Conflicted,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryScores {
    pub confidence: f32,
    pub importance: f32,
    pub stability: f32,
    pub freshness: f32,
}

impl Default for MemoryScores {
    fn default() -> Self {
        Self {
            confidence: 0.6,
            importance: 0.5,
            stability: 0.5,
            freshness: 1.0,
        }
    }
}

impl MemoryScores {
    pub fn validate(self) -> Result<Self, DomainError> {
        for (field, value) in [
            ("confidence", self.confidence),
            ("importance", self.importance),
            ("stability", self.stability),
            ("freshness", self.freshness),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(DomainError::InvalidScore {
                    field,
                    value: value.to_string(),
                });
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: MemoryId,
    pub scope_id: ScopeId,
    pub owner_scope_id: ScopeId,
    pub published_from_scope_id: Option<ScopeId>,
    pub kind: MemoryKind,
    pub state: MemoryState,
    pub title: String,
    pub body: String,
    pub language_code: Option<String>,
    pub scores: MemoryScores,
    pub visibility: Visibility,
    pub sensitivity: Sensitivity,
    pub evidence_count: usize,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Memory {
    pub fn new(
        scope_id: ScopeId,
        kind: MemoryKind,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "memory.title",
            });
        }

        let now = OffsetDateTime::now_utc();

        Ok(Self {
            id: MemoryId::new(),
            owner_scope_id: scope_id.clone(),
            scope_id,
            published_from_scope_id: None,
            kind,
            state: MemoryState::Candidate,
            title,
            body: body.into(),
            language_code: None,
            scores: MemoryScores::default(),
            visibility: Visibility::Private,
            sensitivity: Sensitivity::Internal,
            evidence_count: 0,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn with_scores(mut self, scores: MemoryScores) -> Result<Self, DomainError> {
        self.scores = scores.validate()?;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(self)
    }

    pub fn with_language_code(
        mut self,
        language_code: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let language_code = language_code.into();
        if language_code.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "memory.language_code",
            });
        }
        self.language_code = Some(language_code);
        self.updated_at = OffsetDateTime::now_utc();
        Ok(self)
    }

    pub fn publish_into(&self, target_scope_id: ScopeId, target_visibility: Visibility) -> Self {
        let mut published = self.clone();
        published.scope_id = target_scope_id;
        published.published_from_scope_id = Some(self.scope_id.clone());
        published.visibility = target_visibility;
        published.updated_at = OffsetDateTime::now_utc();
        published
    }

    pub fn activate(&mut self) -> Result<(), DomainError> {
        if !matches!(self.state, MemoryState::Candidate | MemoryState::Conflicted) {
            return Err(DomainError::InvalidTransition {
                from: self.state.as_str(),
                to: MemoryState::Active.as_str(),
            });
        }

        self.state = MemoryState::Active;
        self.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }
}

impl MemoryState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Conflicted => "conflicted",
            Self::Archived => "archived",
            Self::Deleted => "deleted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Memory, MemoryKind, MemoryScores};
    use crate::ScopeId;

    #[test]
    fn rejects_out_of_range_scores() {
        let memory = Memory::new(ScopeId::new(), MemoryKind::Fact, "title", "body").unwrap();
        let scores = MemoryScores {
            confidence: 1.1,
            ..MemoryScores::default()
        };

        assert!(memory.with_scores(scores).is_err());
    }
}
