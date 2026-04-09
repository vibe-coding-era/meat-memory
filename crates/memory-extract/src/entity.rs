use memory_domain::{Entity, EntityType, ScopeId};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EntityCandidate {
    pub entity: Entity,
    pub confidence: f32,
    pub evidence_text: String,
}

pub fn extract_entities(scope_id: &ScopeId, text: &str) -> Vec<EntityCandidate> {
    let mut candidates = HashMap::<String, EntityCandidate>::new();

    for code_symbol in capture_wrapped(text, '`', '`') {
        insert_candidate(
            &mut candidates,
            scope_id,
            EntityType::CodeSymbol,
            &code_symbol,
            0.95,
            &code_symbol,
        );
    }

    for title_case in capture_title_case_phrases(text) {
        insert_candidate(
            &mut candidates,
            scope_id,
            infer_entity_type(text, &title_case),
            &title_case,
            0.72,
            &title_case,
        );
    }

    for contextual in capture_contextual_entities(text) {
        insert_candidate(
            &mut candidates,
            scope_id,
            contextual.entity_type,
            &contextual.name,
            contextual.confidence,
            &contextual.name,
        );
    }

    let mut ordered = candidates.into_values().collect::<Vec<_>>();
    ordered.sort_by(|left, right| right.confidence.total_cmp(&left.confidence));
    ordered
}

struct NamedEntityHint {
    entity_type: EntityType,
    name: String,
    confidence: f32,
}

fn insert_candidate(
    candidates: &mut HashMap<String, EntityCandidate>,
    scope_id: &ScopeId,
    entity_type: EntityType,
    name: &str,
    confidence: f32,
    evidence_text: &str,
) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return;
    }

    let Ok(entity) = Entity::new(scope_id.clone(), entity_type, trimmed) else {
        return;
    };
    if entity.normalized_key.len() < 3 {
        return;
    }

    let key = entity.normalized_key.clone();
    let candidate = EntityCandidate {
        entity,
        confidence,
        evidence_text: evidence_text.to_string(),
    };

    match candidates.get(&key) {
        Some(existing) if existing.confidence >= candidate.confidence => {}
        _ => {
            candidates.insert(key, candidate);
        }
    }
}

fn capture_wrapped(text: &str, open: char, close: char) -> Vec<String> {
    let mut captured = Vec::new();
    let mut current = String::new();
    let mut inside = false;

    for ch in text.chars() {
        if ch == open && !inside {
            inside = true;
            current.clear();
            continue;
        }
        if ch == close && inside {
            inside = false;
            let value = current.trim();
            if !value.is_empty() {
                captured.push(value.to_string());
            }
            current.clear();
            continue;
        }
        if inside {
            current.push(ch);
        }
    }

    captured
}

fn capture_title_case_phrases(text: &str) -> Vec<String> {
    let mut phrases = Vec::new();
    let mut current = Vec::<String>::new();

    for token in text.split_whitespace() {
        let cleaned = trim_token(token);
        if is_title_case_token(cleaned) {
            current.push(cleaned.to_string());
            continue;
        }

        if current.len() >= 2 {
            phrases.push(current.join(" "));
        }
        current.clear();
    }

    if current.len() >= 2 {
        phrases.push(current.join(" "));
    }

    phrases
}

fn capture_contextual_entities(text: &str) -> Vec<NamedEntityHint> {
    let mut hints = Vec::new();
    let normalized = text.replace('\n', " ");

    for (keyword, entity_type, confidence) in [
        ("project ", EntityType::Project, 0.88_f32),
        ("service ", EntityType::Service, 0.86_f32),
        ("repository ", EntityType::Repository, 0.9_f32),
        ("repo ", EntityType::Repository, 0.84_f32),
        ("team ", EntityType::Team, 0.84_f32),
        ("organization ", EntityType::Organization, 0.84_f32),
        ("workspace ", EntityType::Workspace, 0.82_f32),
    ] {
        let lower = normalized.to_lowercase();
        let mut start = 0usize;
        while let Some(offset) = lower[start..].find(keyword) {
            let begin = start + offset + keyword.len();
            let tail = &normalized[begin..];
            let name = tail
                .split(['.', ',', ';', ':', '\n'])
                .next()
                .unwrap_or("")
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            let cleaned = trim_token(&name);
            if cleaned.len() >= 3 {
                hints.push(NamedEntityHint {
                    entity_type,
                    name: cleaned.to_string(),
                    confidence,
                });
            }
            start = begin;
        }
    }

    for (keyword, entity_type) in [
        ("项目", EntityType::Project),
        ("服务", EntityType::Service),
        ("仓库", EntityType::Repository),
        ("团队", EntityType::Team),
    ] {
        let mut start = 0usize;
        while let Some(offset) = normalized[start..].find(keyword) {
            let begin = start + offset + keyword.len();
            let tail = &normalized[begin..];
            let name = tail
                .chars()
                .skip_while(|ch| ch.is_whitespace())
                .take_while(|ch| !matches!(ch, '，' | '。' | '；' | ':' | '\n' | ' ' | ','))
                .collect::<String>();
            let cleaned = trim_token(&name);
            if cleaned.len() >= 2 {
                hints.push(NamedEntityHint {
                    entity_type,
                    name: cleaned.to_string(),
                    confidence: 0.8,
                });
            }
            start = begin;
        }
    }

    hints
}

fn infer_entity_type(text: &str, name: &str) -> EntityType {
    let lowered_text = text.to_lowercase();
    let lowered_name = name.to_lowercase();
    let pattern = format!("project {}", lowered_name);
    if lowered_text.contains(&pattern) {
        return EntityType::Project;
    }
    let pattern = format!("service {}", lowered_name);
    if lowered_text.contains(&pattern) {
        return EntityType::Service;
    }
    let pattern = format!("repository {}", lowered_name);
    if lowered_text.contains(&pattern) || lowered_text.contains(&format!("repo {}", lowered_name)) {
        return EntityType::Repository;
    }
    EntityType::Topic
}

fn is_title_case_token(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_uppercase() {
        return false;
    }
    chars.any(|ch| ch.is_lowercase())
}

fn trim_token(token: &str) -> &str {
    token.trim_matches(|ch: char| {
        matches!(
            ch,
            '.' | ','
                | ';'
                | ':'
                | '"'
                | '\''
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '!'
                | '?'
                | '，'
                | '。'
                | '；'
                | '：'
                | '（'
                | '）'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        capture_contextual_entities, capture_title_case_phrases, capture_wrapped, extract_entities,
        infer_entity_type, insert_candidate, is_title_case_token, trim_token,
    };
    use memory_domain::{EntityType, ScopeId};
    use std::collections::HashMap;

    #[test]
    fn extract_entities_merges_sources_and_sorts_by_confidence() {
        let scope_id = ScopeId::from_string("scp_extract_entities");
        let candidates = extract_entities(
            &scope_id,
            "project Meat Memory, service Gateway Api, workspace Alpha Lab, \
             repository Memory Core. 团队平台组。服务记忆中台。 \
             Use `GatewayClient` for all requests.",
        );

        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].entity.entity_type, EntityType::CodeSymbol);
        assert_eq!(candidates[0].entity.canonical_name, "GatewayClient");
        assert_eq!(candidates[0].evidence_text, "GatewayClient");

        let by_key = candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.entity.normalized_key.clone(),
                    (candidate.entity.entity_type, candidate.confidence),
                )
            })
            .collect::<HashMap<_, _>>();

        assert_eq!(by_key["meatmemory"].0, EntityType::Project);
        assert_eq!(by_key["gatewayapi"].0, EntityType::Service);
        assert_eq!(by_key["memorycore"].0, EntityType::Repository);
        assert_eq!(by_key["alphalab"].0, EntityType::Workspace);
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.entity.canonical_name == "平台组")
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.entity.canonical_name == "记忆中台")
        );
        assert!((by_key["gatewayclient"].1 - 0.95).abs() < f32::EPSILON);
        assert!((by_key["meatmemory"].1 - 0.88).abs() < f32::EPSILON);
    }

    #[test]
    fn insert_candidate_skips_invalid_values_and_keeps_highest_confidence() {
        let scope_id = ScopeId::from_string("scp_insert_candidate");
        let mut candidates = HashMap::new();

        insert_candidate(
            &mut candidates,
            &scope_id,
            EntityType::Topic,
            "   ",
            0.5,
            "blank",
        );
        insert_candidate(
            &mut candidates,
            &scope_id,
            EntityType::Topic,
            "AI",
            0.9,
            "too short",
        );
        assert!(candidates.is_empty());

        insert_candidate(
            &mut candidates,
            &scope_id,
            EntityType::Project,
            "Meat Memory",
            0.7,
            "low",
        );
        insert_candidate(
            &mut candidates,
            &scope_id,
            EntityType::Project,
            "Meat Memory",
            0.6,
            "lower",
        );
        insert_candidate(
            &mut candidates,
            &scope_id,
            EntityType::Project,
            "Meat Memory",
            0.91,
            "higher",
        );

        let candidate = candidates.get("meatmemory").unwrap();
        assert_eq!(candidate.entity.entity_type, EntityType::Project);
        assert!((candidate.confidence - 0.91).abs() < f32::EPSILON);
        assert_eq!(candidate.evidence_text, "higher");
    }

    #[test]
    fn helper_extractors_cover_wrapped_title_case_and_contextual_patterns() {
        assert_eq!(
            capture_wrapped(
                "Use `GatewayClient` and `MemorySync`; ignore `` and `open",
                '`',
                '`'
            ),
            vec!["GatewayClient".to_string(), "MemorySync".to_string()]
        );
        assert_eq!(
            capture_title_case_phrases(
                "the Meat Memory platform integrates Service Gateway and leaves api lowercase"
            ),
            vec!["Meat Memory".to_string(), "Service Gateway".to_string()]
        );

        let contextual = capture_contextual_entities(
            "project Meat Memory, service Gateway Api, repository Memory Core, \
             repo Edge Sync, team Platform Infra, organization Memory Org, workspace Alpha Lab. \
             project AI should skip. 项目记忆系统。 服务记忆网关。 仓库核心仓。 团队平台组。 团队 A",
        );
        let names = contextual
            .iter()
            .map(|hint| (hint.entity_type, hint.name.clone()))
            .collect::<Vec<_>>();

        assert!(names.contains(&(EntityType::Project, "Meat Memory".to_string())));
        assert!(names.contains(&(EntityType::Service, "Gateway Api".to_string())));
        assert!(names.contains(&(EntityType::Repository, "Memory Core".to_string())));
        assert!(names.contains(&(EntityType::Repository, "Edge Sync".to_string())));
        assert!(names.contains(&(EntityType::Team, "Platform Infra".to_string())));
        assert!(names.contains(&(EntityType::Organization, "Memory Org".to_string())));
        assert!(names.contains(&(EntityType::Workspace, "Alpha Lab".to_string())));
        assert!(names.contains(&(EntityType::Project, "记忆系统".to_string())));
        assert!(names.contains(&(EntityType::Service, "记忆网关".to_string())));
        assert!(names.contains(&(EntityType::Repository, "核心仓".to_string())));
        assert!(names.contains(&(EntityType::Team, "平台组".to_string())));
        assert!(!names.iter().any(|(_, name)| name == "AI"));
        assert!(!names.iter().any(|(_, name)| name == "A"));
    }

    #[test]
    fn infer_entity_type_and_token_helpers_cover_fallbacks() {
        assert_eq!(
            infer_entity_type(
                "project Meat Memory uses service Gateway Api",
                "Meat Memory"
            ),
            EntityType::Project
        );
        assert_eq!(
            infer_entity_type("service Gateway Api handles routing", "Gateway Api"),
            EntityType::Service
        );
        assert_eq!(
            infer_entity_type("repo Memory Core stores state", "Memory Core"),
            EntityType::Repository
        );
        assert_eq!(
            infer_entity_type("general topic mention", "Loose Topic"),
            EntityType::Topic
        );

        assert!(is_title_case_token("Gateway"));
        assert!(!is_title_case_token(""));
        assert!(!is_title_case_token("gateway"));
        assert!(!is_title_case_token("API"));
        assert_eq!(trim_token("（Meat Memory），"), "Meat Memory");
        assert_eq!(trim_token("[Gateway Api]."), "Gateway Api");
    }
}
