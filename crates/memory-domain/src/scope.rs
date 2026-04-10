use crate::{DomainError, ScopeId, Visibility};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScopeType {
    Org,
    Team,
    Workspace,
    Project,
    User,
    Session,
}

impl ScopeType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Org => "org",
            Self::Team => "team",
            Self::Workspace => "workspace",
            Self::Project => "project",
            Self::User => "user",
            Self::Session => "session",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InheritPolicy {
    Inherit,
    Isolated,
}

impl InheritPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Isolated => "isolated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncPolicy {
    LocalOnly,
    Shared,
    Replicated,
}

impl SyncPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalOnly => "local_only",
            Self::Shared => "shared",
            Self::Replicated => "replicated",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub id: ScopeId,
    pub scope_type: ScopeType,
    pub name: String,
    pub path: String,
    pub parent_scope_id: Option<ScopeId>,
    pub owner_principal_id: Option<String>,
    pub inherit_policy: InheritPolicy,
    pub default_visibility: Visibility,
    pub sync_policy: SyncPolicy,
}

impl Scope {
    pub fn new(
        scope_type: ScopeType,
        name: impl Into<String>,
        path: impl Into<String>,
        parent_scope_id: Option<ScopeId>,
    ) -> Result<Self, DomainError> {
        Self::new_with_id(ScopeId::new(), scope_type, name, path, parent_scope_id)
    }

    pub fn new_with_id(
        id: ScopeId,
        scope_type: ScopeType,
        name: impl Into<String>,
        path: impl Into<String>,
        parent_scope_id: Option<ScopeId>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let path = normalize_path(path.into())?;
        if name.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "scope.name",
            });
        }

        Ok(Self {
            id,
            scope_type,
            name,
            path,
            parent_scope_id,
            owner_principal_id: None,
            inherit_policy: default_inherit_policy(scope_type),
            default_visibility: Visibility::Private,
            sync_policy: default_sync_policy(scope_type),
        })
    }

    pub fn with_owner_principal_id(
        mut self,
        owner_principal_id: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let owner_principal_id = owner_principal_id.into();
        if owner_principal_id.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "scope.owner_principal_id",
            });
        }

        self.owner_principal_id = Some(owner_principal_id);
        Ok(self)
    }

    pub fn with_default_visibility(mut self, default_visibility: Visibility) -> Self {
        self.default_visibility = default_visibility;
        self
    }

    pub fn with_policies(mut self, inherit_policy: InheritPolicy, sync_policy: SyncPolicy) -> Self {
        self.inherit_policy = inherit_policy;
        self.sync_policy = sync_policy;
        self
    }
}

pub struct ScopeHierarchyValidator;

impl ScopeHierarchyValidator {
    pub fn validate(scopes: &[Scope]) -> Result<(), DomainError> {
        let scope_map = scopes
            .iter()
            .map(|scope| (scope.id.as_str().to_string(), scope))
            .collect::<HashMap<_, _>>();
        let parent_map = scopes
            .iter()
            .map(|scope| (scope.id.as_str().to_string(), scope.parent_scope_id.clone()))
            .collect::<HashMap<_, _>>();

        for scope in scopes {
            validate_parent(scope, &scope_map)?;
            let mut visited = HashSet::new();
            let mut current = Some(scope.id.clone());

            while let Some(scope_id) = current {
                let raw_id = scope_id.as_str().to_string();
                if !visited.insert(raw_id.clone()) {
                    return Err(DomainError::ScopeCycle { scope_id: raw_id });
                }

                current = parent_map
                    .get(scope_id.as_str())
                    .and_then(|parent_scope_id| parent_scope_id.clone());
            }
        }

        Ok(())
    }
}

fn normalize_path(path: String) -> Result<String, DomainError> {
    let trimmed = path.trim().trim_matches('/').to_string();
    if trimmed.is_empty() {
        return Err(DomainError::EmptyField {
            field: "scope.path",
        });
    }
    if trimmed.split('/').any(|segment| segment.trim().is_empty()) {
        return Err(DomainError::InvalidField {
            field: "scope.path",
            reason: "path segments cannot be empty",
        });
    }
    Ok(trimmed)
}

fn default_inherit_policy(scope_type: ScopeType) -> InheritPolicy {
    match scope_type {
        ScopeType::User | ScopeType::Session => InheritPolicy::Isolated,
        ScopeType::Org | ScopeType::Team | ScopeType::Workspace | ScopeType::Project => {
            InheritPolicy::Inherit
        }
    }
}

fn default_sync_policy(scope_type: ScopeType) -> SyncPolicy {
    match scope_type {
        ScopeType::Session => SyncPolicy::LocalOnly,
        ScopeType::User => SyncPolicy::Shared,
        ScopeType::Org | ScopeType::Team | ScopeType::Workspace | ScopeType::Project => {
            SyncPolicy::Replicated
        }
    }
}

fn validate_parent(scope: &Scope, scope_map: &HashMap<String, &Scope>) -> Result<(), DomainError> {
    let Some(parent_scope_id) = &scope.parent_scope_id else {
        if matches!(scope.scope_type, ScopeType::Session) {
            return Err(DomainError::InvalidScopeHierarchy {
                parent_scope_type: "root",
                child_scope_type: scope.scope_type.as_str(),
            });
        }
        return Ok(());
    };

    let Some(parent) = scope_map.get(parent_scope_id.as_str()) else {
        return Err(DomainError::MissingScopeParent {
            scope_id: scope.id.as_str().to_string(),
            parent_scope_id: parent_scope_id.as_str().to_string(),
        });
    };

    if !is_allowed_parent(parent.scope_type, scope.scope_type) {
        return Err(DomainError::InvalidScopeHierarchy {
            parent_scope_type: parent.scope_type.as_str(),
            child_scope_type: scope.scope_type.as_str(),
        });
    }

    let expected_prefix = format!("{}/", parent.path);
    if !scope.path.starts_with(&expected_prefix) {
        return Err(DomainError::InvalidScopePath {
            scope_id: scope.id.as_str().to_string(),
            expected_prefix,
        });
    }

    Ok(())
}

fn is_allowed_parent(parent_scope_type: ScopeType, child_scope_type: ScopeType) -> bool {
    match child_scope_type {
        ScopeType::Org => false,
        ScopeType::Team => matches!(parent_scope_type, ScopeType::Org),
        ScopeType::Workspace => matches!(parent_scope_type, ScopeType::Org | ScopeType::Team),
        ScopeType::Project => matches!(
            parent_scope_type,
            ScopeType::Workspace | ScopeType::Team | ScopeType::User
        ),
        ScopeType::User => matches!(
            parent_scope_type,
            ScopeType::Org | ScopeType::Team | ScopeType::Workspace | ScopeType::Project
        ),
        ScopeType::Session => matches!(parent_scope_type, ScopeType::User | ScopeType::Project),
    }
}

#[cfg(test)]
mod tests {
    use super::{InheritPolicy, Scope, ScopeHierarchyValidator, ScopeType, SyncPolicy};
    use crate::{DomainError, Visibility};

    #[test]
    fn accepts_linear_scope_tree() {
        let org = Scope::new(ScopeType::Org, "Acme", "org/acme", None).unwrap();
        let team = Scope::new(
            ScopeType::Team,
            "Platform",
            "org/acme/team/platform",
            Some(org.id.clone()),
        )
        .unwrap();

        assert!(ScopeHierarchyValidator::validate(&[org, team]).is_ok());
    }

    #[test]
    fn scope_defaults_reflect_v2_isolation_intent() {
        let project = Scope::new(
            ScopeType::Project,
            "Project A",
            "workspace/a/project/a",
            None,
        )
        .unwrap();
        let user = Scope::new(ScopeType::User, "Alice", "workspace/a/user/alice", None).unwrap();
        let session = Scope::new(
            ScopeType::Session,
            "Session 1",
            "workspace/a/user/alice/session/1",
            Some(user.id.clone()),
        )
        .unwrap();

        assert_eq!(project.inherit_policy, InheritPolicy::Inherit);
        assert_eq!(project.sync_policy, SyncPolicy::Replicated);
        assert_eq!(user.inherit_policy, InheritPolicy::Isolated);
        assert_eq!(user.sync_policy, SyncPolicy::Shared);
        assert_eq!(session.sync_policy, SyncPolicy::LocalOnly);
    }

    #[test]
    fn rejects_missing_parent_scope_reference() {
        let scope = Scope::new(
            ScopeType::Project,
            "Project A",
            "workspace/a/project/a",
            Some(crate::ScopeId::from_string("scp_missing")),
        )
        .unwrap();

        let error = ScopeHierarchyValidator::validate(&[scope]).unwrap_err();
        assert!(matches!(error, DomainError::MissingScopeParent { .. }));
    }

    #[test]
    fn rejects_scope_paths_that_escape_parent_path() {
        let org = Scope::new(ScopeType::Org, "Acme", "org/acme", None).unwrap();
        let team = Scope::new(
            ScopeType::Team,
            "Platform",
            "org/other/team/platform",
            Some(org.id.clone()),
        )
        .unwrap();

        let error = ScopeHierarchyValidator::validate(&[org, team]).unwrap_err();
        assert!(matches!(error, DomainError::InvalidScopePath { .. }));
    }

    #[test]
    fn rejects_session_scope_without_parent() {
        let session = Scope::new(
            ScopeType::Session,
            "Session 1",
            "workspace/a/session/1",
            None,
        )
        .unwrap();

        let error = ScopeHierarchyValidator::validate(&[session]).unwrap_err();
        assert!(matches!(error, DomainError::InvalidScopeHierarchy { .. }));
    }

    #[test]
    fn owner_builder_and_visibility_overrides_work() {
        let scope = Scope::new(ScopeType::Team, "Platform", "org/acme/team/platform", None)
            .unwrap()
            .with_owner_principal_id("usr_alice")
            .unwrap()
            .with_default_visibility(Visibility::Team)
            .with_policies(InheritPolicy::Inherit, SyncPolicy::Replicated);

        assert_eq!(scope.owner_principal_id.as_deref(), Some("usr_alice"));
        assert_eq!(scope.default_visibility, Visibility::Team);
    }
}
