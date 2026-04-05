use crate::{DomainError, EntityId, EvidenceId, RelationId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    MemberOf,
    BelongsTo,
    Owns,
    DependsOn,
    Uses,
    Implements,
    References,
    DerivedFrom,
    Documents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationState {
    Candidate,
    Active,
    Rejected,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: RelationId,
    pub relation_type: RelationType,
    pub subject_entity_id: EntityId,
    pub object_entity_id: EntityId,
    pub state: RelationState,
    pub evidence_ids: Vec<EvidenceId>,
    pub created_at: OffsetDateTime,
}

impl Relation {
    pub fn new(
        relation_type: RelationType,
        subject_entity_id: EntityId,
        object_entity_id: EntityId,
    ) -> Self {
        Self {
            id: RelationId::new(),
            relation_type,
            subject_entity_id,
            object_entity_id,
            state: RelationState::Candidate,
            evidence_ids: Vec::new(),
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn add_evidence(&mut self, evidence_id: EvidenceId) {
        self.evidence_ids.push(evidence_id);
    }

    pub fn activate(&mut self) -> Result<(), DomainError> {
        if self.evidence_ids.is_empty() {
            return Err(DomainError::MissingEvidence);
        }

        self.state = RelationState::Active;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Relation, RelationType};
    use crate::EntityId;

    #[test]
    fn refuses_to_activate_without_evidence() {
        let mut relation = Relation::new(RelationType::DependsOn, EntityId::new(), EntityId::new());
        assert!(relation.activate().is_err());
    }
}
