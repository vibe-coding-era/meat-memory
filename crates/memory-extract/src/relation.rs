use crate::entity::EntityCandidate;
use memory_domain::{Relation, RelationType, ScopeId};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RelationCandidate {
    pub relation: Relation,
    pub confidence: f32,
    pub evidence_text: String,
}

pub fn extract_relations(
    _scope_id: &ScopeId,
    text: &str,
    entities: &[EntityCandidate],
) -> Vec<RelationCandidate> {
    let mut candidates = HashMap::<String, RelationCandidate>::new();
    let lowered_text = text.to_lowercase();

    for subject in entities {
        for object in entities {
            if subject.entity.id == object.entity.id {
                continue;
            }

            for (keyword, relation_type, confidence) in [
                ("depends on", RelationType::DependsOn, 0.93_f32),
                ("uses", RelationType::Uses, 0.9_f32),
                ("belongs to", RelationType::BelongsTo, 0.88_f32),
                ("references", RelationType::References, 0.86_f32),
                ("implements", RelationType::Implements, 0.89_f32),
                ("documents", RelationType::Documents, 0.84_f32),
                ("依赖", RelationType::DependsOn, 0.93_f32),
                ("使用", RelationType::Uses, 0.9_f32),
                ("属于", RelationType::BelongsTo, 0.88_f32),
                ("引用", RelationType::References, 0.86_f32),
                ("实现", RelationType::Implements, 0.89_f32),
                ("记录", RelationType::Documents, 0.84_f32),
                ("文档化", RelationType::Documents, 0.84_f32),
            ] {
                if contains_relation_phrase(
                    &lowered_text,
                    &subject.entity.canonical_name.to_lowercase(),
                    keyword,
                    &object.entity.canonical_name.to_lowercase(),
                ) {
                    let relation = Relation::new(
                        relation_type,
                        subject.entity.id.clone(),
                        object.entity.id.clone(),
                    );
                    let key = format!(
                        "{:?}:{}:{}",
                        relation_type, subject.entity.normalized_key, object.entity.normalized_key
                    );
                    let candidate = RelationCandidate {
                        relation,
                        confidence,
                        evidence_text: text.to_string(),
                    };
                    match candidates.get(&key) {
                        Some(existing) if existing.confidence >= candidate.confidence => {}
                        _ => {
                            candidates.insert(key, candidate);
                        }
                    }
                }
            }
        }
    }

    let mut ordered = candidates.into_values().collect::<Vec<_>>();
    ordered.sort_by(|left, right| right.confidence.total_cmp(&left.confidence));
    ordered
}

fn contains_relation_phrase(text: &str, subject: &str, keyword: &str, object: &str) -> bool {
    let keyword = keyword.trim();
    let joined = format!("{subject}{keyword}{object}");
    let spaced = format!("{subject} {keyword} {object}");
    let wrapped_joined = format!("`{subject}`{keyword}`{object}`");
    let wrapped_spaced = format!("`{subject}` {keyword} `{object}`");

    text.contains(&joined)
        || text.contains(&spaced)
        || text.contains(&wrapped_joined)
        || text.contains(&wrapped_spaced)
}

#[cfg(test)]
mod tests {
    use super::{contains_relation_phrase, extract_relations};
    use crate::entity::EntityCandidate;
    use memory_domain::{Entity, EntityId, EntityType, RelationType, ScopeId};

    #[test]
    fn contains_relation_phrase_matches_plain_and_wrapped_patterns() {
        assert!(contains_relation_phrase(
            "gateway depends on redis",
            "gateway",
            "depends on",
            "redis"
        ));
        assert!(contains_relation_phrase(
            "`gateway` uses `redis`",
            "gateway",
            "uses",
            "redis"
        ));
        let lowered = "`服务网关` 依赖 `PostgreSQL`".to_lowercase();
        assert!(contains_relation_phrase(
            &lowered,
            "服务网关",
            "依赖",
            "postgresql"
        ));
        assert!(!contains_relation_phrase(
            "gateway talks with redis",
            "gateway",
            "uses",
            "redis"
        ));
    }

    #[test]
    fn extract_relations_detects_and_sorts_relation_candidates() {
        let scope_id = ScopeId::from_string("scp_rel");
        let gateway = candidate(&scope_id, "Gateway", "gateway");
        let redis = candidate(&scope_id, "Redis", "redis");
        let docs = candidate(&scope_id, "Runbook", "runbook");

        let relations = extract_relations(
            &scope_id,
            "Gateway depends on Redis and Gateway documents Runbook.",
            &[gateway, redis, docs],
        );

        assert_eq!(relations.len(), 2);
        assert_eq!(relations[0].relation.relation_type, RelationType::DependsOn);
        assert_eq!(relations[0].confidence, 0.93);
        assert_eq!(relations[1].relation.relation_type, RelationType::Documents);
        assert_eq!(relations[1].confidence, 0.84);
        assert_eq!(
            relations[0].evidence_text,
            "Gateway depends on Redis and Gateway documents Runbook."
        );
    }

    #[test]
    fn extract_relations_supports_chinese_relation_keywords() {
        let scope_id = ScopeId::from_string("scp_rel_zh");
        let gateway = candidate(&scope_id, "服务网关", "服务网关");
        let postgres = candidate(&scope_id, "PostgreSQL", "postgresql");
        let runbook = candidate(&scope_id, "发布手册", "发布手册");

        let relations = extract_relations(
            &scope_id,
            "`服务网关` 依赖 `PostgreSQL`，`服务网关` 记录 `发布手册`。",
            &[gateway, postgres, runbook],
        );

        assert_eq!(relations.len(), 2);
        assert_eq!(relations[0].relation.relation_type, RelationType::DependsOn);
        assert_eq!(relations[1].relation.relation_type, RelationType::Documents);
    }

    #[test]
    fn extract_relations_deduplicates_candidates_with_same_normalized_keys() {
        let scope_id = ScopeId::from_string("scp_rel");
        let gateway_a = candidate_with_id(&scope_id, "Gateway", "gateway", "ent_gateway_a");
        let gateway_b = candidate_with_id(&scope_id, "Gateway", "gateway", "ent_gateway_b");
        let redis = candidate_with_id(&scope_id, "Redis", "redis", "ent_redis");

        let relations = extract_relations(
            &scope_id,
            "Gateway uses Redis.",
            &[gateway_a, gateway_b, redis],
        );

        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].relation.relation_type, RelationType::Uses);
    }

    #[test]
    fn extract_relations_skips_self_relations_and_non_matching_text() {
        let scope_id = ScopeId::from_string("scp_rel");
        let gateway = candidate_with_id(&scope_id, "Gateway", "gateway", "ent_gateway");
        let same_gateway = candidate_with_id(&scope_id, "Gateway", "gateway", "ent_gateway");

        let relations = extract_relations(
            &scope_id,
            "Gateway collaborates with Gateway.",
            &[gateway, same_gateway],
        );

        assert!(relations.is_empty());
    }

    fn candidate(
        scope_id: &ScopeId,
        canonical_name: &str,
        normalized_key: &str,
    ) -> EntityCandidate {
        candidate_with_id(
            scope_id,
            canonical_name,
            normalized_key,
            &format!("ent_{normalized_key}"),
        )
    }

    fn candidate_with_id(
        scope_id: &ScopeId,
        canonical_name: &str,
        normalized_key: &str,
        entity_id: &str,
    ) -> EntityCandidate {
        let mut entity = Entity::new(scope_id.clone(), EntityType::Service, canonical_name)
            .expect("entity should build");
        entity.id = EntityId::from_string(entity_id);
        entity.normalized_key = normalized_key.to_string();

        EntityCandidate {
            entity,
            confidence: 0.9,
            evidence_text: canonical_name.to_string(),
        }
    }
}
