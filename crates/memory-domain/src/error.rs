use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("{field} cannot be empty")]
    EmptyField { field: &'static str },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("invalid state transition from {from} to {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("score {field} must be within [0.0, 1.0], got {value}")]
    InvalidScore { field: &'static str, value: String },
    #[error("active relations must include at least one evidence id")]
    MissingEvidence,
    #[error("scope {scope_id} references missing parent {parent_scope_id}")]
    MissingScopeParent {
        scope_id: String,
        parent_scope_id: String,
    },
    #[error("scope hierarchy contains a cycle involving {scope_id}")]
    ScopeCycle { scope_id: String },
    #[error("scope hierarchy forbids {child_scope_type} under {parent_scope_type}")]
    InvalidScopeHierarchy {
        parent_scope_type: &'static str,
        child_scope_type: &'static str,
    },
    #[error("scope {scope_id} path must stay under {expected_prefix}")]
    InvalidScopePath {
        scope_id: String,
        expected_prefix: String,
    },
}
