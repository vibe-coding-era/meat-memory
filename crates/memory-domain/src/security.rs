use crate::{MemoryId, ScopeId, SecretFindingId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretFindingKind {
    ApiKey,
    BearerToken,
    PrivateKey,
    Password,
    Email,
    PhoneNumber,
    PrivatePath,
}

impl SecretFindingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::BearerToken => "bearer_token",
            Self::PrivateKey => "private_key",
            Self::Password => "password",
            Self::Email => "email",
            Self::PhoneNumber => "phone_number",
            Self::PrivatePath => "private_path",
        }
    }

    pub fn redaction_label(self) -> &'static str {
        match self {
            Self::ApiKey => "API_KEY",
            Self::BearerToken => "BEARER_TOKEN",
            Self::PrivateKey => "PRIVATE_KEY",
            Self::Password => "PASSWORD",
            Self::Email => "EMAIL",
            Self::PhoneNumber => "PHONE_NUMBER",
            Self::PrivatePath => "PRIVATE_PATH",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretFindingAction {
    Allow,
    Redact,
    Proposal,
    Deny,
    RecallBlock,
}

impl SecretFindingAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Redact => "redact",
            Self::Proposal => "proposal",
            Self::Deny => "deny",
            Self::RecallBlock => "recall_block",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretRiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl SecretRiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretFindingLocation {
    pub field: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretFinding {
    pub id: SecretFindingId,
    pub scope_id: ScopeId,
    pub memory_id: Option<MemoryId>,
    pub source_ref: String,
    pub kind: SecretFindingKind,
    pub action: SecretFindingAction,
    pub risk_level: SecretRiskLevel,
    pub fingerprint: String,
    pub redacted_preview: String,
    pub location: SecretFindingLocation,
    pub created_at: OffsetDateTime,
}

impl SecretFinding {
    pub fn new(
        scope_id: ScopeId,
        memory_id: Option<MemoryId>,
        source_ref: impl Into<String>,
        kind: SecretFindingKind,
        action: SecretFindingAction,
        risk_level: SecretRiskLevel,
        location: SecretFindingLocation,
        matched_text: &str,
    ) -> Self {
        Self {
            id: SecretFindingId::new(),
            scope_id,
            memory_id,
            source_ref: source_ref.into(),
            kind,
            action,
            risk_level,
            fingerprint: fingerprint(matched_text),
            redacted_preview: format!("[REDACTED:{}]", kind.redaction_label()),
            location,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn with_action(mut self, action: SecretFindingAction) -> Self {
        self.action = action;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryHealthRiskKind {
    Duplicate,
    Conflict,
    Stale,
    LowConfidence,
    HighSensitivity,
    Unreviewed,
    OrphanEvidence,
    SecretFinding,
}

impl MemoryHealthRiskKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate",
            Self::Conflict => "conflict",
            Self::Stale => "stale",
            Self::LowConfidence => "low_confidence",
            Self::HighSensitivity => "high_sensitivity",
            Self::Unreviewed => "unreviewed",
            Self::OrphanEvidence => "orphan_evidence",
            Self::SecretFinding => "secret_finding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryHealthSeverity {
    Info,
    Warning,
    Critical,
}

impl MemoryHealthSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryHealthSuggestedAction {
    Keep,
    Review,
    Merge,
    Redact,
    AddEvidence,
    Archive,
}

impl MemoryHealthSuggestedAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Review => "review",
            Self::Merge => "merge",
            Self::Redact => "redact",
            Self::AddEvidence => "add_evidence",
            Self::Archive => "archive",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryHealthRisk {
    pub kind: MemoryHealthRiskKind,
    pub severity: MemoryHealthSeverity,
    pub memory_id: Option<MemoryId>,
    pub title: Option<String>,
    pub detail: String,
    pub suggested_action: MemoryHealthSuggestedAction,
}

fn fingerprint(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().fold(
        String::with_capacity(digest.len() * 2),
        |mut output, byte| {
            let _ = write!(&mut output, "{byte:02x}");
            output
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        MemoryHealthRiskKind, MemoryHealthSeverity, MemoryHealthSuggestedAction, SecretFinding,
        SecretFindingAction, SecretFindingKind, SecretFindingLocation, SecretRiskLevel,
    };
    use crate::ScopeId;

    #[test]
    fn secret_enums_expose_stable_labels() {
        assert_eq!(SecretFindingKind::ApiKey.as_str(), "api_key");
        assert_eq!(
            SecretFindingKind::BearerToken.redaction_label(),
            "BEARER_TOKEN"
        );
        assert_eq!(SecretFindingAction::RecallBlock.as_str(), "recall_block");
        assert_eq!(SecretRiskLevel::Critical.as_str(), "critical");
        assert_eq!(
            MemoryHealthRiskKind::SecretFinding.as_str(),
            "secret_finding"
        );
        assert_eq!(MemoryHealthSeverity::Warning.as_str(), "warning");
        assert_eq!(
            MemoryHealthSuggestedAction::AddEvidence.as_str(),
            "add_evidence"
        );
    }

    #[test]
    fn finding_stores_fingerprint_without_raw_secret() {
        let finding = SecretFinding::new(
            ScopeId::from_string("scp_secret"),
            None,
            "artifact:art_1",
            SecretFindingKind::ApiKey,
            SecretFindingAction::Redact,
            SecretRiskLevel::High,
            SecretFindingLocation {
                field: "body".to_string(),
                start: 4,
                end: 20,
            },
            "sk_test_very_secret",
        );

        assert_eq!(finding.redacted_preview, "[REDACTED:API_KEY]");
        assert_ne!(finding.fingerprint, "sk_test_very_secret");
        assert_eq!(finding.fingerprint.len(), 64);
    }
}
