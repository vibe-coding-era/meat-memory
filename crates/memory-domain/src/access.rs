use crate::{AccessKeyId, DomainError, ScopeId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeySourceKind {
    Cli,
    Mcp,
    Skill,
    Http,
    Tui,
    Custom,
}

impl KeySourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Mcp => "mcp",
            Self::Skill => "skill",
            Self::Http => "http",
            Self::Tui => "tui",
            Self::Custom => "custom",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "cli" => Ok(Self::Cli),
            "mcp" => Ok(Self::Mcp),
            "skill" => Ok(Self::Skill),
            "http" => Ok(Self::Http),
            "tui" => Ok(Self::Tui),
            "custom" => Ok(Self::Custom),
            _ => Err(DomainError::InvalidField {
                field: "access_key.source_kind",
                reason: "unsupported source kind",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyScopeKind {
    Personal,
    Team,
}

impl KeyScopeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Team => "team",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "personal" => Ok(Self::Personal),
            "team" => Ok(Self::Team),
            _ => Err(DomainError::InvalidField {
                field: "access_key.scope_kind",
                reason: "unsupported scope kind",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageMode {
    File,
    Vector,
    All,
}

impl StorageMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Vector => "vector",
            Self::All => "all",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "file" => Ok(Self::File),
            "vector" => Ok(Self::Vector),
            "all" => Ok(Self::All),
            _ => Err(DomainError::InvalidField {
                field: "access_key.storage_mode",
                reason: "unsupported storage mode",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessKeyStatus {
    Active,
    Disabled,
    Revoked,
}

impl AccessKeyStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Revoked => "revoked",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw {
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            "revoked" => Ok(Self::Revoked),
            _ => Err(DomainError::InvalidField {
                field: "access_key.status",
                reason: "unsupported key status",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessKey {
    pub id: AccessKeyId,
    pub key_hash: String,
    pub display_name: String,
    pub source_kind: KeySourceKind,
    pub owner_principal_id: String,
    pub owner_scope_id: ScopeId,
    pub scope_kind: KeyScopeKind,
    pub storage_mode: StorageMode,
    pub is_fully_isolated: bool,
    pub isolation_group_id: String,
    pub status: AccessKeyStatus,
    pub created_at: OffsetDateTime,
    pub last_used_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyUsageBreakdown {
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessKeyUsageStats {
    pub key_id: AccessKeyId,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: u64,
    pub last_used_at: Option<OffsetDateTime>,
    pub by_source: Vec<KeyUsageBreakdown>,
    pub by_storage_mode: Vec<KeyUsageBreakdown>,
}

impl AccessKey {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        raw_key: &str,
        display_name: impl Into<String>,
        source_kind: KeySourceKind,
        owner_principal_id: impl Into<String>,
        owner_scope_id: ScopeId,
        scope_kind: KeyScopeKind,
        storage_mode: StorageMode,
        is_fully_isolated: bool,
    ) -> Result<Self, DomainError> {
        let id = AccessKeyId::new();
        Self::new_with_id(
            id,
            raw_key,
            display_name,
            source_kind,
            owner_principal_id,
            owner_scope_id,
            scope_kind,
            storage_mode,
            is_fully_isolated,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_id(
        id: AccessKeyId,
        raw_key: &str,
        display_name: impl Into<String>,
        source_kind: KeySourceKind,
        owner_principal_id: impl Into<String>,
        owner_scope_id: ScopeId,
        scope_kind: KeyScopeKind,
        storage_mode: StorageMode,
        is_fully_isolated: bool,
    ) -> Result<Self, DomainError> {
        let display_name = non_empty(display_name.into(), "access_key.display_name")?;
        let owner_principal_id =
            non_empty(owner_principal_id.into(), "access_key.owner_principal_id")?;
        let key_hash = hash_access_key(raw_key)?;
        let isolation_group_id = match scope_kind {
            KeyScopeKind::Personal => format!("personal:{}", owner_principal_id),
            KeyScopeKind::Team => format!("team:{}", owner_scope_id.as_str()),
        };
        let now = OffsetDateTime::now_utc();

        Ok(Self {
            id,
            key_hash,
            display_name,
            source_kind,
            owner_principal_id,
            owner_scope_id,
            scope_kind,
            storage_mode,
            is_fully_isolated,
            isolation_group_id,
            status: AccessKeyStatus::Active,
            created_at: now,
            last_used_at: None,
        })
    }

    pub fn to_context(&self) -> RequestContext {
        RequestContext {
            key_id: self.id.clone(),
            source_kind: self.source_kind,
            principal_id: self.owner_principal_id.clone(),
            owner_scope_id: self.owner_scope_id.clone(),
            scope_kind: self.scope_kind,
            storage_mode: self.storage_mode,
            is_fully_isolated: self.is_fully_isolated,
            isolation_group_id: self.isolation_group_id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestContext {
    pub key_id: AccessKeyId,
    pub source_kind: KeySourceKind,
    pub principal_id: String,
    pub owner_scope_id: ScopeId,
    pub scope_kind: KeyScopeKind,
    pub storage_mode: StorageMode,
    pub is_fully_isolated: bool,
    pub isolation_group_id: String,
}

pub fn hash_access_key(raw_key: &str) -> Result<String, DomainError> {
    if raw_key.trim().is_empty() {
        return Err(DomainError::EmptyField {
            field: "access_key.raw_key",
        });
    }
    let digest = Sha256::digest(raw_key.as_bytes());
    Ok(format!("{digest:x}"))
}

fn non_empty(value: String, field: &'static str) -> Result<String, DomainError> {
    if value.trim().is_empty() {
        return Err(DomainError::EmptyField { field });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{AccessKey, KeyScopeKind, KeySourceKind, StorageMode, hash_access_key};
    use crate::ScopeId;

    #[test]
    fn builds_access_key_context_without_storing_raw_secret() {
        let key = AccessKey::new(
            "mmk_secret",
            "codex local",
            KeySourceKind::Cli,
            "rou",
            ScopeId::from_string("scp_user_rou"),
            KeyScopeKind::Personal,
            StorageMode::All,
            false,
        )
        .unwrap();

        assert_ne!(key.key_hash, "mmk_secret");
        assert_eq!(key.scope_kind, KeyScopeKind::Personal);
        assert_eq!(key.storage_mode, StorageMode::All);
        assert_eq!(key.to_context().isolation_group_id, "personal:rou");
    }

    #[test]
    fn hash_rejects_empty_secret() {
        assert!(hash_access_key(" ").is_err());
    }
}
