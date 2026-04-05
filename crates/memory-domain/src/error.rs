use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("{field} cannot be empty")]
    EmptyField { field: &'static str },
    #[error("invalid state transition from {from} to {to}")]
    InvalidTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("score {field} must be within [0.0, 1.0], got {value}")]
    InvalidScore { field: &'static str, value: String },
    #[error("active relations must include at least one evidence id")]
    MissingEvidence,
    #[error("scope hierarchy contains a cycle involving {scope_id}")]
    ScopeCycle { scope_id: String },
}
