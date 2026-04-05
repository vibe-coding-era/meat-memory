use crate::{Entity, Memory, Relation, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    pub query: String,
    pub scope_id: ScopeId,
    pub memories: Vec<Memory>,
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    pub generated_at: OffsetDateTime,
}

impl ContextBundle {
    pub fn empty(query: impl Into<String>, scope_id: ScopeId) -> Self {
        Self {
            query: query.into(),
            scope_id,
            memories: Vec::new(),
            entities: Vec::new(),
            relations: Vec::new(),
            generated_at: OffsetDateTime::now_utc(),
        }
    }
}
