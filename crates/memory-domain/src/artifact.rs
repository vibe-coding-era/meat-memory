use crate::{ArtifactId, DomainError, ScopeId, Sensitivity, Visibility};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactKind {
    Message,
    Document,
    CodeDiff,
    CodeFileSnapshot,
    TerminalOutput,
    Image,
    Audio,
    Video,
    ToolResult,
    WebPage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub scope_id: ScopeId,
    pub kind: ArtifactKind,
    pub mime_type: Option<String>,
    pub language_code: Option<String>,
    pub content_text: String,
    pub content_hash: String,
    pub source_refs: Vec<String>,
    pub labels: Vec<String>,
    pub visibility: Visibility,
    pub sensitivity: Sensitivity,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl Artifact {
    pub fn new(
        scope_id: ScopeId,
        kind: ArtifactKind,
        content_text: impl Into<String>,
        source_refs: Vec<String>,
    ) -> Result<Self, DomainError> {
        let content_text = content_text.into();
        if content_text.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "artifact.content_text",
            });
        }

        let now = OffsetDateTime::now_utc();

        Ok(Self {
            id: ArtifactId::new(),
            scope_id,
            kind,
            mime_type: None,
            language_code: None,
            content_hash: Self::compute_content_hash(&content_text),
            content_text,
            source_refs,
            labels: Vec::new(),
            visibility: Visibility::Private,
            sensitivity: Sensitivity::Internal,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn compute_content_hash(content: &str) -> String {
        let digest = Sha256::digest(content.as_bytes());
        digest.iter().fold(
            String::with_capacity(digest.len() * 2),
            |mut output, byte| {
                let _ = write!(&mut output, "{byte:02x}");
                output
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Artifact, ArtifactKind};
    use crate::ScopeId;

    #[test]
    fn computes_stable_hash_for_same_content() {
        let artifact_a = Artifact::new(
            ScopeId::new(),
            ArtifactKind::Message,
            "remember me",
            vec!["session://1".to_string()],
        )
        .unwrap();
        let artifact_b = Artifact::new(
            ScopeId::new(),
            ArtifactKind::Message,
            "remember me",
            vec!["session://2".to_string()],
        )
        .unwrap();

        assert_eq!(artifact_a.content_hash, artifact_b.content_hash);
    }
}
