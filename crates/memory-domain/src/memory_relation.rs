use crate::{DomainError, MemoryId, MemoryRelationId, ProposalId, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRelationType {
    Supersedes,
    ConflictsWith,
    DerivedFrom,
    MergedInto,
    RelatedTo,
}

impl MemoryRelationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supersedes => "supersedes",
            Self::ConflictsWith => "conflicts_with",
            Self::DerivedFrom => "derived_from",
            Self::MergedInto => "merged_into",
            Self::RelatedTo => "related_to",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRelationSourceKind {
    User,
    Agent,
    System,
}

impl MemoryRelationSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRelation {
    pub id: MemoryRelationId,
    pub scope_id: ScopeId,
    pub from_memory_id: MemoryId,
    pub to_memory_id: MemoryId,
    pub relation_type: MemoryRelationType,
    pub confidence: f32,
    pub source_kind: MemoryRelationSourceKind,
    pub source_proposal_id: Option<ProposalId>,
    pub created_at: OffsetDateTime,
}

impl MemoryRelation {
    pub fn new(
        scope_id: ScopeId,
        from_memory_id: MemoryId,
        to_memory_id: MemoryId,
        relation_type: MemoryRelationType,
        source_kind: MemoryRelationSourceKind,
    ) -> Self {
        Self {
            id: MemoryRelationId::new(),
            scope_id,
            from_memory_id,
            to_memory_id,
            relation_type,
            confidence: 1.0,
            source_kind,
            source_proposal_id: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Result<Self, DomainError> {
        self.confidence = validate_confidence(confidence, "memory_relation.confidence")?;
        Ok(self)
    }

    pub fn with_source_proposal_id(mut self, proposal_id: ProposalId) -> Self {
        self.source_proposal_id = Some(proposal_id);
        self
    }
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
    use super::{MemoryRelation, MemoryRelationSourceKind, MemoryRelationType};
    use crate::{MemoryId, ProposalId, ScopeId};

    #[test]
    fn memory_relation_defaults_and_tracks_source_proposal() {
        let relation = MemoryRelation::new(
            ScopeId::from_string("scp_v28"),
            MemoryId::from_string("mem_new"),
            MemoryId::from_string("mem_old"),
            MemoryRelationType::Supersedes,
            MemoryRelationSourceKind::System,
        )
        .with_source_proposal_id(ProposalId::from_string("prp_123"));

        assert!(relation.id.as_str().starts_with("mrl_"));
        assert_eq!(relation.relation_type.as_str(), "supersedes");
        assert_eq!(relation.source_kind.as_str(), "system");
        assert_eq!(
            relation.source_proposal_id.as_ref().map(|id| id.as_str()),
            Some("prp_123")
        );
        assert_eq!(relation.confidence, 1.0);
    }

    #[test]
    fn memory_relation_validates_confidence() {
        let relation = MemoryRelation::new(
            ScopeId::from_string("scp_v28"),
            MemoryId::from_string("mem_left"),
            MemoryId::from_string("mem_right"),
            MemoryRelationType::ConflictsWith,
            MemoryRelationSourceKind::User,
        );

        assert!(relation.clone().with_confidence(0.6).is_ok());
        assert!(relation.with_confidence(1.2).is_err());
    }

    #[test]
    fn relation_type_and_source_kind_render_expected_strings() {
        assert_eq!(MemoryRelationType::Supersedes.as_str(), "supersedes");
        assert_eq!(MemoryRelationType::ConflictsWith.as_str(), "conflicts_with");
        assert_eq!(MemoryRelationType::DerivedFrom.as_str(), "derived_from");
        assert_eq!(MemoryRelationType::MergedInto.as_str(), "merged_into");
        assert_eq!(MemoryRelationType::RelatedTo.as_str(), "related_to");
        assert_eq!(MemoryRelationSourceKind::User.as_str(), "user");
        assert_eq!(MemoryRelationSourceKind::Agent.as_str(), "agent");
        assert_eq!(MemoryRelationSourceKind::System.as_str(), "system");
    }
}
