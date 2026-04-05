use memory_domain::{EntityType, RelationType, ScopeId};
use memory_extract::{ExtractionEnvelope, extract_entities, extract_relations, should_extract};

#[test]
fn empty_payloads_do_not_enter_pipeline() {
    let envelope = ExtractionEnvelope::new("message", "   \n  ");
    assert!(!should_extract(&envelope));
}

#[test]
fn low_signal_text_produces_no_entity_or_relation_candidates() {
    let scope_id = ScopeId::from_string("scp_extract_low_signal");
    let text = "keep this short and generic";

    let entities = extract_entities(&scope_id, text);
    let relations = extract_relations(&scope_id, text, &entities);

    assert!(entities.is_empty());
    assert!(relations.is_empty());
}

#[test]
fn multilingual_text_extracts_contextual_and_code_entities() {
    let scope_id = ScopeId::from_string("scp_extract_multilingual");
    let text = "Project Meat Memory 通过 服务网关 调用 `search_context` 工具。";

    let entities = extract_entities(&scope_id, text);

    assert!(
        entities
            .iter()
            .any(|candidate| candidate.entity.normalized_key == "projectmeatmemory")
    );
    assert!(entities.iter().any(|candidate| {
        candidate.entity.entity_type == EntityType::Service
            && candidate.entity.canonical_name.contains("网关")
    }));
    assert!(entities.iter().any(|candidate| {
        candidate.entity.entity_type == EntityType::CodeSymbol
            && candidate.entity.canonical_name == "search_context"
    }));
}

#[test]
fn english_relation_phrase_produces_uses_relation() {
    let scope_id = ScopeId::from_string("scp_extract_relation");
    let text = "Project Meat Memory uses Service Gateway for context routing.";

    let entities = extract_entities(&scope_id, text);
    let relations = extract_relations(&scope_id, text, &entities);

    assert!(
        relations
            .iter()
            .any(|candidate| candidate.relation.relation_type == RelationType::Uses)
    );
}
