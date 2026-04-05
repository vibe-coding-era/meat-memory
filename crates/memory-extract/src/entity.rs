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
