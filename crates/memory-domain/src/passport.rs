use crate::{MemoryPassportId, ScopeId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use time::OffsetDateTime;

pub const MEMORY_PASSPORT_SCHEMA_VERSION: &str = "2.94";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPassportObjectKind {
    Memory,
    EvidenceSpan,
}

impl MemoryPassportObjectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::EvidenceSpan => "evidence_span",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPassportRedaction {
    None,
    Sensitive,
}

impl MemoryPassportRedaction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sensitive => "sensitive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPassportEncryption {
    pub enabled: bool,
    pub algorithm: Option<String>,
    pub key_ref: Option<String>,
}

impl MemoryPassportEncryption {
    pub fn none() -> Self {
        Self {
            enabled: false,
            algorithm: None,
            key_ref: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPassportObject {
    pub kind: MemoryPassportObjectKind,
    pub id: String,
    pub hash: String,
    pub byte_len: usize,
}

impl MemoryPassportObject {
    pub fn new(
        kind: MemoryPassportObjectKind,
        id: impl Into<String>,
        payload: impl AsRef<[u8]>,
    ) -> Self {
        let payload = payload.as_ref();
        Self {
            kind,
            id: id.into(),
            hash: stable_hash_bytes(payload),
            byte_len: payload.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPassportManifest {
    pub id: MemoryPassportId,
    pub schema_version: String,
    pub source_scope_id: ScopeId,
    pub object_count: usize,
    pub redaction: MemoryPassportRedaction,
    pub encryption: MemoryPassportEncryption,
    pub objects: Vec<MemoryPassportObject>,
    pub bundle_hash: String,
    pub exported_at: OffsetDateTime,
}

impl MemoryPassportManifest {
    pub fn new(
        source_scope_id: ScopeId,
        mut objects: Vec<MemoryPassportObject>,
        redaction: MemoryPassportRedaction,
        encryption: MemoryPassportEncryption,
        exported_at: OffsetDateTime,
    ) -> Self {
        objects.sort_by(|left, right| {
            left.kind
                .as_str()
                .cmp(right.kind.as_str())
                .then_with(|| left.id.cmp(&right.id))
        });
        let object_count = objects.len();
        let bundle_hash = compute_bundle_hash(&source_scope_id, &objects, redaction, &encryption);
        Self {
            id: MemoryPassportId::new(),
            schema_version: MEMORY_PASSPORT_SCHEMA_VERSION.to_string(),
            source_scope_id,
            object_count,
            redaction,
            encryption,
            objects,
            bundle_hash,
            exported_at,
        }
    }
}

pub fn stable_hash(value: &str) -> String {
    stable_hash_bytes(value.as_bytes())
}

fn stable_hash_bytes(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    digest.iter().fold(
        String::with_capacity(digest.len() * 2),
        |mut output, byte| {
            let _ = write!(&mut output, "{byte:02x}");
            output
        },
    )
}

fn compute_bundle_hash(
    scope_id: &ScopeId,
    objects: &[MemoryPassportObject],
    redaction: MemoryPassportRedaction,
    encryption: &MemoryPassportEncryption,
) -> String {
    let mut input = format!(
        "{}\n{}\n{}\n{}\n",
        MEMORY_PASSPORT_SCHEMA_VERSION,
        scope_id.as_str(),
        redaction.as_str(),
        encryption.enabled
    );
    for object in objects {
        input.push_str(object.kind.as_str());
        input.push('\t');
        input.push_str(&object.id);
        input.push('\t');
        input.push_str(&object.hash);
        input.push('\t');
        input.push_str(&object.byte_len.to_string());
        input.push('\n');
    }
    stable_hash(&input)
}

#[cfg(test)]
mod tests {
    use super::{
        MEMORY_PASSPORT_SCHEMA_VERSION, MemoryPassportEncryption, MemoryPassportManifest,
        MemoryPassportObject, MemoryPassportObjectKind, MemoryPassportRedaction, stable_hash,
    };
    use crate::ScopeId;
    use time::macros::datetime;

    #[test]
    fn passport_labels_and_hashes_are_stable() {
        assert_eq!(MemoryPassportObjectKind::Memory.as_str(), "memory");
        assert_eq!(
            MemoryPassportObjectKind::EvidenceSpan.as_str(),
            "evidence_span"
        );
        assert_eq!(MemoryPassportRedaction::Sensitive.as_str(), "sensitive");
        assert_eq!(stable_hash("same"), stable_hash("same"));
    }

    #[test]
    fn manifest_sorts_objects_and_computes_bundle_hash() {
        let manifest = MemoryPassportManifest::new(
            ScopeId::from_string("scp_passport"),
            vec![
                MemoryPassportObject::new(
                    MemoryPassportObjectKind::EvidenceSpan,
                    "evd_b",
                    "evidence",
                ),
                MemoryPassportObject::new(MemoryPassportObjectKind::Memory, "mem_a", "memory"),
            ],
            MemoryPassportRedaction::None,
            MemoryPassportEncryption::none(),
            datetime!(2026-04-27 00:00 UTC),
        );

        assert_eq!(manifest.schema_version, MEMORY_PASSPORT_SCHEMA_VERSION);
        assert_eq!(manifest.object_count, 2);
        assert_eq!(manifest.objects[0].id, "evd_b");
        assert_eq!(manifest.objects[1].id, "mem_a");
        assert_eq!(manifest.bundle_hash.len(), 64);
        assert!(!manifest.encryption.enabled);
    }
}
