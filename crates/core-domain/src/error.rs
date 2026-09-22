//! Domain Error Definitions
//!
//! Strongly-typed domain errors representing violations of business logic,
//! state machines, integrity checks, and validation invariants.

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Integrity check failed: expected {expected}, got {actual}")]
    IntegrityCheckFailed { expected: String, actual: String },

    #[error("Budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Entity not found: {kind} with ID {id}")]
    NotFound { kind: String, id: String },

    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}
