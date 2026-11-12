//! Memory Value Objects (Custos.md §9.3)
//!
//! Pure data types used by `custos_core::contracts::MemoryPort`.
//! The model may READ memory and PROPOSE facts; it can never write or delete directly.
//!
//! STATUS: DRAFT under Interface Freeze (Sprint 0). Field changes require an RFC.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Identifier of the worker run that produced a proposal.
pub type WorkerRunId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallQuery {
    pub text: String,
    pub limit: usize,
    /// Optional token ceiling so the Context Compiler can budget recalled memory.
    pub token_budget: Option<usize>,
}

/// Which memory zones a recall may touch. Defaults to the narrowest scope.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryScope {
    pub task_id: Option<String>,
    pub project: Option<String>,
    pub include_personal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub source: String,
    pub score: f32,
    pub recorded_at: DateTime<Utc>,
}

/// A temporal fact: valid over [valid_from, valid_to).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactProposal {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    /// CAS hashes / evidence ids supporting the proposal.
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalReceipt {
    pub proposal_id: String,
    pub status: ProposalStatus,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MemoryError {
    #[error("Memory entry not found: {0}")]
    NotFound(String),
    #[error("Memory proposal rejected: {0}")]
    Rejected(String),
    #[error("Memory backend failure: {0}")]
    Backend(String),
}
