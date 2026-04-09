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

#[cfg(test)]
mod tests {
    use super::ContextBundle;
    use crate::ScopeId;
    use time::OffsetDateTime;

    #[test]
    fn empty_bundle_initializes_without_context_items() {
        let scope_id = ScopeId::from_string("scp_ctx");
        let before = OffsetDateTime::now_utc();

        let bundle = ContextBundle::empty("搜索网关", scope_id.clone());

        let after = OffsetDateTime::now_utc();
        assert_eq!(bundle.query, "搜索网关");
        assert_eq!(bundle.scope_id, scope_id);
        assert!(bundle.memories.is_empty());
        assert!(bundle.entities.is_empty());
        assert!(bundle.relations.is_empty());
        assert!(bundle.generated_at >= before);
        assert!(bundle.generated_at <= after);
    }
}
