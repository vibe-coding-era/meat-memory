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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub id: ScopeId,
    pub scope_type: ScopeType,
    pub name: String,
    pub path: String,
    pub parent_scope_id: Option<ScopeId>,
    pub default_visibility: Visibility,
}

impl Scope {
    pub fn new(
        scope_type: ScopeType,
        name: impl Into<String>,
        path: impl Into<String>,
        parent_scope_id: Option<ScopeId>,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let path = path.into();
        if name.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "scope.name",
            });
        }
        if path.trim().is_empty() {
            return Err(DomainError::EmptyField {
                field: "scope.path",
            });
        }

        Ok(Self {
            id: ScopeId::new(),
            scope_type,
            name,
            path,
            parent_scope_id,
            default_visibility: Visibility::Private,
        })
    }
}

pub struct ScopeHierarchyValidator;

impl ScopeHierarchyValidator {
    pub fn validate(scopes: &[Scope]) -> Result<(), DomainError> {
        let parent_map = scopes
            .iter()
            .map(|scope| (scope.id.as_str().to_string(), scope.parent_scope_id.clone()))
            .collect::<HashMap<_, _>>();

        for scope in scopes {
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

#[cfg(test)]
mod tests {
    use super::{Scope, ScopeHierarchyValidator, ScopeType};

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
}
