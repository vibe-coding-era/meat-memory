use crate::{DomainError, EntityId, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Person,
    Team,
    Organization,
    Workspace,
    Project,
    Repository,
    Service,
    Document,
    Task,
    Topic,
    CodeSymbol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub scope_id: ScopeId,
    pub entity_type: EntityType,
    pub canonical_name: String,
    pub description: Option<String>,
    pub aliases: Vec<String>,
    pub normalized_key: String,
    pub created_at: OffsetDateTime,
}

impl Entity {
    pub fn new(
        scope_id: ScopeId,
        entity_type: EntityType,
        canonical_name: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let canonical_name = canonical_name.into();
        if canonical_name.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "entity.canonical_name",
            });
        }

        Ok(Self {
            id: EntityId::new(),
            scope_id,
            entity_type,
            normalized_key: Self::normalized_key_for(&canonical_name),
            canonical_name,
            description: None,
            aliases: Vec::new(),
            created_at: OffsetDateTime::now_utc(),
        })
    }

    pub fn normalized_key_for(input: &str) -> String {
        input
            .chars()
            .filter(|char| char.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    }
}

#[cfg(test)]
mod tests {
    use super::{Entity, EntityType};
    use crate::ScopeId;

    #[test]
    fn creates_deterministic_normalized_key() {
        let entity = Entity::new(ScopeId::new(), EntityType::Project, "Meat Memory").unwrap();
        assert_eq!(entity.normalized_key, "meatmemory");
    }
}
