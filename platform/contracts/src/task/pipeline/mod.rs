//! Universal Sovereign Data Pipeline Domain Models
//!
//! Defines the core types and invariants for the 3-stage data lifecycle (Bronze -> Silver -> Gold)
//! across all 4 Custos Workbenches:
//! - `Code` (Coding Workbench)
//! - `Science` (Research Workbench)
//! - `Agent` (Assistant Workbench)
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
    /// Coding Workbench: Source code, AST symbol DAGs, diffs, terminal PTY logs.
    Code,
    /// Research Workbench: Literature papers, claims, evidence anchors, molecular structures.
    Science,
    /// Assistant Workbench: Agent conversations, context packs, tool-call traces, outbox actions.
    Agent,
    /// Quant Workbench: OHLCV time-series bars, ticks, factors, order tickets, tearsheets.
    Market,
}

/// Invariant verification contract for domain-specific integrity rules.
pub trait DataInvariant: Send + Sync {
    /// Verify that the data satisfies strict domain invariants before promoting to Silver/Gold.
    fn verify(&self) -> Result<(), DomainError>;
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

    struct AlwaysValidInvariant;
    impl DataInvariant for AlwaysValidInvariant {
        fn verify(&self) -> Result<(), DomainError> {
            Ok(())
        }
    }

    #[test]
    fn test_pipeline_record_lifecycle_bronze_silver_gold() {
        let payload = "{\"symbol\":\"BTC/USDT\",\"close\":68500.0}";
        let record = PipelineRecord::new_bronze("rec_1", DataModality::Market, payload, 1000);
        assert_eq!(record.stage, PipelineStage::Bronze);
        assert!(!record.is_verified);

        let silver = record
            .promote_to_silver(&AlwaysValidInvariant)
            .expect("silver promotion succeeds");
        assert_eq!(silver.stage, PipelineStage::Silver);
        assert!(silver.is_verified);

        let gold = silver.promote_to_gold().expect("gold promotion succeeds");
        assert_eq!(gold.stage, PipelineStage::Gold);
    }
}
