use crate::{ArtifactId, EvidenceId, MemoryId, ScopeId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceSpanKind {
    Text,
    ImageRegion,
    AudioSegment,
    VideoSegment,
}

impl EvidenceSpanKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::ImageRegion => "image_region",
            Self::AudioSegment => "audio_segment",
            Self::VideoSegment => "video_segment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSpanLocation {
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub page: Option<u32>,
    pub region: Option<[f32; 4]>,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
}

impl EvidenceSpanLocation {
    pub fn text(start: usize, end: usize) -> Self {
        Self {
            start: Some(start),
            end: Some(end),
            page: None,
            region: None,
            start_ms: None,
            end_ms: None,
        }
    }

    pub fn media() -> Self {
        Self {
            start: None,
            end: None,
            page: None,
            region: None,
            start_ms: Some(0),
            end_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSpan {
    pub id: EvidenceId,
    pub scope_id: ScopeId,
    pub memory_id: Option<MemoryId>,
    pub artifact_id: Option<ArtifactId>,
    pub source_ref: String,
    pub kind: EvidenceSpanKind,
    pub quote: String,
    pub content_hash: String,
    pub location: EvidenceSpanLocation,
    pub created_at: OffsetDateTime,
}

impl EvidenceSpan {
    pub fn new_text(
        scope_id: ScopeId,
        memory_id: Option<MemoryId>,
        artifact_id: Option<ArtifactId>,
        source_ref: impl Into<String>,
        quote: impl Into<String>,
        content_hash: impl Into<String>,
        start: usize,
        end: usize,
    ) -> Self {
        Self {
            id: EvidenceId::new(),
            scope_id,
            memory_id,
            artifact_id,
            source_ref: source_ref.into(),
            kind: EvidenceSpanKind::Text,
            quote: quote.into(),
            content_hash: content_hash.into(),
            location: EvidenceSpanLocation::text(start, end),
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn new_media(
        scope_id: ScopeId,
        memory_id: Option<MemoryId>,
        artifact_id: Option<ArtifactId>,
        source_ref: impl Into<String>,
        kind: EvidenceSpanKind,
        quote: impl Into<String>,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            id: EvidenceId::new(),
            scope_id,
            memory_id,
            artifact_id,
            source_ref: source_ref.into(),
            kind,
            quote: quote.into(),
            content_hash: content_hash.into(),
            location: EvidenceSpanLocation::media(),
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EvidenceSpan, EvidenceSpanKind, EvidenceSpanLocation};
    use crate::{ArtifactId, MemoryId, ScopeId};

    #[test]
    fn evidence_span_kinds_expose_stable_labels() {
        assert_eq!(EvidenceSpanKind::Text.as_str(), "text");
        assert_eq!(EvidenceSpanKind::ImageRegion.as_str(), "image_region");
        assert_eq!(EvidenceSpanKind::AudioSegment.as_str(), "audio_segment");
        assert_eq!(EvidenceSpanKind::VideoSegment.as_str(), "video_segment");
    }

    #[test]
    fn text_evidence_span_keeps_source_location() {
        let span = EvidenceSpan::new_text(
            ScopeId::from_string("scp_evidence"),
            Some(MemoryId::from_string("mem_evidence")),
            Some(ArtifactId::from_string("art_evidence")),
            "artifact:art_evidence",
            "release note",
            "sha256:test",
            4,
            16,
        );

        assert_eq!(span.source_ref, "artifact:art_evidence");
        assert_eq!(span.kind, EvidenceSpanKind::Text);
        assert_eq!(span.location, EvidenceSpanLocation::text(4, 16));
    }
}
