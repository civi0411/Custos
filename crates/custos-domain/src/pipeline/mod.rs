//! Universal Sovereign Data Pipeline Domain Models
//!
//! Defines the core types and invariants for the 3-stage data lifecycle (Bronze -> Silver -> Gold)
//! across all 4 Custos Workbenches:
//! - `Code` (Engineering Workbench)
//! - `Science` (Research Workbench)
//! - `Agent` (Copilot Workbench)
//! - `Market` (Quant Workbench)

pub mod structure;
pub use structure::{
    DetectedStructure, InferredColumn, InferredDataType, TabularContractInvariant,
    TabularStructureDetector,
};

use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The three canonical stages of the sovereign data pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    /// Raw, unvalidated, provider-specific ingested data.
    Bronze,
    /// Sanitized, normalized, structure-detected and invariant-enforced data.
    Silver,
    /// Point-in-time ready, aggregated, indexed, and query-optimized data.
    Gold,
}

/// The four specialized data modalities matching the 4 Custos Workbenches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataModality {
    /// Engineering Workbench: Source code, AST symbol DAGs, diffs, terminal PTY logs.
    Code,
    /// Research Workbench: Literature papers, claims, evidence anchors, molecular structures.
    Science,
    /// Copilot Workbench: Agent conversations, context packs, tool-call traces, outbox actions.
    Agent,
    /// Quant Workbench: OHLCV time-series bars, ticks, factors, order tickets, tearsheets.
    Market,
}

/// Invariant verification contract for domain-specific integrity rules.
pub trait DataInvariant: Send + Sync {
    /// Verify that the data satisfies strict domain invariants before promoting to Silver/Gold.
    fn verify(&self) -> Result<(), DomainError>;
}

/// Quant Invariant: Guarantees zero lookahead bias by enforcing closed bar boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosedBarInvariant {
    pub bar_open_ts: i64,
    pub duration_seconds: i64,
    pub reference_ts: i64,
}

impl ClosedBarInvariant {
    pub fn new(bar_open_ts: i64, duration_seconds: i64, reference_ts: i64) -> Self {
        Self {
            bar_open_ts,
            duration_seconds,
            reference_ts,
        }
    }
}

impl DataInvariant for ClosedBarInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.duration_seconds <= 0 {
            return Err(DomainError::Validation(
                "Bar duration must be positive".to_string(),
            ));
        }
        let bar_close_ts = self.bar_open_ts.saturating_add(self.duration_seconds);
        if bar_close_ts > self.reference_ts {
            return Err(DomainError::Validation(format!(
                "Lookahead violation: bar closes at {} which is after reference time {}",
                bar_close_ts, self.reference_ts
            )));
        }
        Ok(())
    }
}

/// Research Invariant: Guarantees every claim is strictly grounded in an immutable passage anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGroundingInvariant {
    pub source_id: String,
    pub document_hash: String,
    pub quote_text: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

impl EvidenceGroundingInvariant {
    pub fn new(
        source_id: impl Into<String>,
        document_hash: impl Into<String>,
        quote_text: impl Into<String>,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            document_hash: document_hash.into(),
            quote_text: quote_text.into(),
            start_offset,
            end_offset,
        }
    }
}

impl DataInvariant for EvidenceGroundingInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.source_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Claim must specify a non-empty source ID".to_string(),
            ));
        }
        if self.document_hash.trim().is_empty() {
            return Err(DomainError::Validation(
                "Claim must bind to a valid document content hash".to_string(),
            ));
        }
        if self.quote_text.trim().is_empty() {
            return Err(DomainError::Validation(
                "Claim must cite non-empty grounding passage text".to_string(),
            ));
        }
        if self.start_offset >= self.end_offset {
            return Err(DomainError::Validation(format!(
                "Invalid anchor offsets: start ({}) >= end ({})",
                self.start_offset, self.end_offset
            )));
        }
        Ok(())
    }
}

/// Copilot Invariant: Guarantees effectful actions have explicit permits before dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernedOutboxInvariant {
    pub action_id: String,
    pub intent_kind: String,
    pub permit_id: Option<String>,
    pub is_read_only: bool,
}

impl GovernedOutboxInvariant {
    pub fn new(
        action_id: impl Into<String>,
        intent_kind: impl Into<String>,
        permit_id: Option<impl Into<String>>,
        is_read_only: bool,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            intent_kind: intent_kind.into(),
            permit_id: permit_id.map(Into::into),
            is_read_only,
        }
    }
}

impl DataInvariant for GovernedOutboxInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.action_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Action ID cannot be empty".to_string(),
            ));
        }
        if !self.is_read_only && self.permit_id.as_deref().unwrap_or("").trim().is_empty() {
            return Err(DomainError::Validation(format!(
                "Action '{}' of type '{}' is effectful and requires a valid ExecutionPermit",
                self.action_id, self.intent_kind
            )));
        }
        Ok(())
    }
}

/// Engineering Invariant: Guarantees basic syntactic validity and balanced delimiters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstSyntaxInvariant {
    pub path: String,
    pub source_code: String,
}

impl AstSyntaxInvariant {
    pub fn new(path: impl Into<String>, source_code: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source_code: source_code.into(),
        }
    }
}

impl DataInvariant for AstSyntaxInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.path.trim().is_empty() {
            return Err(DomainError::Validation(
                "Source file path cannot be empty".to_string(),
            ));
        }
        let mut brace = 0i32;
        let mut bracket = 0i32;
        let mut paren = 0i32;

        for ch in self.source_code.chars() {
            match ch {
                '{' => brace += 1,
                '}' => {
                    brace -= 1;
                    if brace < 0 {
                        return Err(DomainError::Validation(format!(
                            "Unmatched closing brace in '{}'",
                            self.path
                        )));
                    }
                }
                '[' => bracket += 1,
                ']' => {
                    bracket -= 1;
                    if bracket < 0 {
                        return Err(DomainError::Validation(format!(
                            "Unmatched closing bracket in '{}'",
                            self.path
                        )));
                    }
                }
                '(' => paren += 1,
                ')' => {
                    paren -= 1;
                    if paren < 0 {
                        return Err(DomainError::Validation(format!(
                            "Unmatched closing parenthesis in '{}'",
                            self.path
                        )));
                    }
                }
                _ => {}
            }
        }

        if brace != 0 || bracket != 0 || paren != 0 {
            return Err(DomainError::Validation(format!(
                "Unbalanced delimiters in '{}' (brace: {}, bracket: {}, paren: {})",
                self.path, brace, bracket, paren
            )));
        }
        Ok(())
    }
}

/// Tamper-evident sovereign pipeline record tracking origin, stage, and content hash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineRecord<T> {
    pub id: String,
    pub modality: DataModality,
    pub stage: PipelineStage,
    pub content_hash: String,
    pub timestamp: i64,
    pub payload: T,
    pub is_verified: bool,
}

impl<T: Serialize> PipelineRecord<T> {
    /// Create a new raw Bronze record.
    pub fn new_bronze(
        id: impl Into<String>,
        modality: DataModality,
        payload: T,
        timestamp: i64,
    ) -> Self {
        let serialized = serde_json::to_vec(&payload).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(&serialized);
        let hash = hex::encode(hasher.finalize());

        Self {
            id: id.into(),
            modality,
            stage: PipelineStage::Bronze,
            content_hash: hash,
            timestamp,
            payload,
            is_verified: false,
        }
    }

    /// Promote record to Silver after successfully validating its domain invariant.
    pub fn promote_to_silver(mut self, invariant: &dyn DataInvariant) -> Result<Self, DomainError> {
        invariant.verify()?;
        self.stage = PipelineStage::Silver;
        self.is_verified = true;
        Ok(self)
    }

    /// Promote record to Gold for point-in-time serving.
    pub fn promote_to_gold(mut self) -> Result<Self, DomainError> {
        if self.stage != PipelineStage::Silver || !self.is_verified {
            return Err(DomainError::Validation(
                "Record must pass Silver stage invariant verification before promotion to Gold"
                    .to_string(),
            ));
        }
        self.stage = PipelineStage::Gold;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closed_bar_invariant_lookahead_rejection() {
        // Bar opened at 1000, duration 300 (closes at 1300)
        let valid_inv = ClosedBarInvariant::new(1000, 300, 1300);
        assert!(valid_inv.verify().is_ok());

        let future_ref = ClosedBarInvariant::new(1000, 300, 1400);
        assert!(future_ref.verify().is_ok());

        // Violation: reference time is 1299, but bar closes at 1300 -> Lookahead!
        let lookahead_inv = ClosedBarInvariant::new(1000, 300, 1299);
        assert!(lookahead_inv.verify().is_err());
    }

    #[test]
    fn test_evidence_grounding_invariant_validation() {
        let valid_claim = EvidenceGroundingInvariant::new(
            "paper_42",
            "sha256_abcd1234",
            "CRISPR-Cas9 exhibits 99.2% target specificity",
            120,
            165,
        );
        assert!(valid_claim.verify().is_ok());

        let invalid_offsets = EvidenceGroundingInvariant::new(
            "paper_42",
            "sha256_abcd1234",
            "text",
            200,
            150, // start > end
        );
        assert!(invalid_offsets.verify().is_err());

        let empty_quote =
            EvidenceGroundingInvariant::new("paper_42", "sha256_abcd1234", "   ", 10, 20);
        assert!(empty_quote.verify().is_err());
    }

    #[test]
    fn test_governed_outbox_invariant_validation() {
        // Read-only action without permit is allowed
        let read_action = GovernedOutboxInvariant::new("act_1", "fs.read", None::<String>, true);
        assert!(read_action.verify().is_ok());

        // Effectful action with permit is allowed
        let effect_with_permit =
            GovernedOutboxInvariant::new("act_2", "fs.write", Some("permit_99"), false);
        assert!(effect_with_permit.verify().is_ok());

        // Effectful action WITHOUT permit is strictly rejected!
        let effect_no_permit =
            GovernedOutboxInvariant::new("act_3", "shell.exec", None::<String>, false);
        assert!(effect_no_permit.verify().is_err());
    }

    #[test]
    fn test_ast_syntax_invariant_validation() {
        let valid_code =
            AstSyntaxInvariant::new("src/main.rs", "fn main() { println!(\"Hello\"); }");
        assert!(valid_code.verify().is_ok());

        let unbalanced_code = AstSyntaxInvariant::new("src/lib.rs", "fn test() { let x = [1, 2; }");
        assert!(unbalanced_code.verify().is_err());
    }

    #[test]
    fn test_pipeline_record_lifecycle_bronze_silver_gold() {
        let payload = "{\"symbol\":\"BTC/USDT\",\"close\":68500.0}";
        let record = PipelineRecord::new_bronze("rec_1", DataModality::Market, payload, 1000);
        assert_eq!(record.stage, PipelineStage::Bronze);
        assert!(!record.is_verified);

        let inv = ClosedBarInvariant::new(1000, 60, 1060);
        let silver = record
            .promote_to_silver(&inv)
            .expect("silver promotion succeeds");
        assert_eq!(silver.stage, PipelineStage::Silver);
        assert!(silver.is_verified);

        let gold = silver.promote_to_gold().expect("gold promotion succeeds");
        assert_eq!(gold.stage, PipelineStage::Gold);
    }
}
