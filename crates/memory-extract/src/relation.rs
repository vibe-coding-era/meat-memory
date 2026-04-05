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
                (" depends on ", RelationType::DependsOn, 0.93_f32),
                (" uses ", RelationType::Uses, 0.9_f32),
                (" belongs to ", RelationType::BelongsTo, 0.88_f32),
                (" references ", RelationType::References, 0.86_f32),
                (" implements ", RelationType::Implements, 0.89_f32),
                (" documents ", RelationType::Documents, 0.84_f32),
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
    text.contains(&format!("{subject}{keyword}{object}"))
        || text.contains(&format!("`{subject}`{keyword}`{object}`"))
}
