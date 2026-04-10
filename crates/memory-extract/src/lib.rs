pub mod entity;
pub mod relation;

use anyhow::Result;
pub use entity::{EntityCandidate, extract_entities};
use memory_domain::{Artifact, ArtifactKind, Memory, MemoryKind};
pub use relation::{RelationCandidate, extract_relations};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionEnvelope {
    pub source_kind: String,
    pub text: String,
}

impl ExtractionEnvelope {
    pub fn new(source_kind: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            source_kind: source_kind.into(),
            text: text.into(),
        }
    }
}

pub fn should_extract(envelope: &ExtractionEnvelope) -> bool {
    !envelope.text.trim().is_empty()
}

pub fn distill_candidate_memory(
    artifact: &Artifact,
    title_override: Option<&str>,
    memory_kind_override: Option<MemoryKind>,
) -> Result<Memory> {
    let title = title_override
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| derive_title(&artifact.content_text));

    let mut memory = Memory::new(
        artifact.scope_id.clone(),
        memory_kind_override.unwrap_or_else(|| infer_memory_kind(artifact)),
        title,
        artifact.content_text.clone(),
    )?;
    memory.owner_scope_id = artifact.scope_id.clone();
    memory.language_code = artifact
        .language_code
        .clone()
        .or_else(|| detect_language_code(&artifact.content_text));
    memory.visibility = artifact.visibility;
    memory.sensitivity = artifact.sensitivity;
    Ok(memory)
}

fn derive_title(text: &str) -> String {
    let candidate = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("untitled-memory");

    const MAX_TITLE_CHARS: usize = 72;
    let mut title = candidate.chars().take(MAX_TITLE_CHARS).collect::<String>();
    if candidate.chars().count() > MAX_TITLE_CHARS {
        title.push_str("...");
    }
    title
}

fn infer_memory_kind(artifact: &Artifact) -> MemoryKind {
    let normalized = artifact.content_text.to_lowercase();
    if normalized.contains("prefer") || normalized.contains("preference") {
        return MemoryKind::Preference;
    }
    if artifact.content_text.contains("偏好") || artifact.content_text.contains("更喜欢") {
        return MemoryKind::Preference;
    }
    if normalized.contains("decide") || normalized.contains("decision") {
        return MemoryKind::Decision;
    }
    if artifact.content_text.contains("决定") || artifact.content_text.contains("决策") {
        return MemoryKind::Decision;
    }
    if normalized.contains("procedure") || normalized.contains("runbook") {
        return MemoryKind::Procedure;
    }
    if artifact.content_text.contains("步骤") || artifact.content_text.contains("流程") {
        return MemoryKind::Procedure;
    }
    if normalized.contains("must") || normalized.contains("constraint") {
        return MemoryKind::Constraint;
    }
    if artifact.content_text.contains("必须") || artifact.content_text.contains("约束") {
        return MemoryKind::Constraint;
    }
    if normalized.contains("risk") {
        return MemoryKind::Risk;
    }
    if artifact.content_text.contains("风险") {
        return MemoryKind::Risk;
    }

    match artifact.kind {
        ArtifactKind::Document | ArtifactKind::WebPage => MemoryKind::Fact,
        ArtifactKind::Message | ArtifactKind::TerminalOutput | ArtifactKind::ToolResult => {
            MemoryKind::Summary
        }
        _ => MemoryKind::Insight,
    }
}

pub fn detect_language_code(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut ascii_letters = 0usize;
    let mut cjk_chars = 0usize;
    for ch in trimmed.chars() {
        if ch.is_ascii_alphabetic() {
            ascii_letters += 1;
        } else if ('\u{4E00}'..='\u{9FFF}').contains(&ch) {
            cjk_chars += 1;
        }
    }

    if cjk_chars > ascii_letters {
        Some("zh-CN".to_string())
    } else if ascii_letters > 0 {
        Some("en".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExtractionEnvelope, detect_language_code, distill_candidate_memory, should_extract,
    };
    use memory_domain::{Artifact, ArtifactKind, MemoryKind, ScopeId};

    #[test]
    fn flags_non_empty_payloads() {
        let envelope = ExtractionEnvelope::new("message", "remember this");
        assert!(should_extract(&envelope));
    }

    #[test]
    fn distills_title_and_kind_from_artifact() {
        let artifact = Artifact::new(
            ScopeId::new(),
            ArtifactKind::Message,
            "We decided to keep PostgreSQL as the canonical store.",
            vec!["session://1".to_string()],
        )
        .unwrap();

        let memory = distill_candidate_memory(&artifact, None, None).unwrap();

        assert_eq!(memory.kind, MemoryKind::Decision);
        assert!(memory.title.contains("We decided"));
        assert_eq!(memory.language_code.as_deref(), Some("en"));
    }

    #[test]
    fn detects_cjk_and_procedure_keywords() {
        let artifact = Artifact::new(
            ScopeId::new(),
            ArtifactKind::Document,
            "发布流程：先执行 smoke，再执行回滚检查。",
            vec!["session://2".to_string()],
        )
        .unwrap();

        let memory = distill_candidate_memory(&artifact, None, None).unwrap();
        assert_eq!(memory.kind, MemoryKind::Procedure);
        assert_eq!(memory.language_code.as_deref(), Some("zh-CN"));
        assert_eq!(detect_language_code("hello world").as_deref(), Some("en"));
    }
}
