//! Verifiable Research Claim, Source, Passage Anchor, and Lineage Domain Models
//! Strictly enforces Invariant Grounding Levels (L0-L3) and Cryptographic Proofs.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

/// Legacy Claim representation for backwards compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub id: String,
    pub statement: String,
    pub supported_by: Vec<String>,
    pub refuted_by: Vec<String>,
}

/// 4-Level Grounding Status enforcing zero false-passes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimGroundingLevel {
    /// L0: Raw assertion without citation or verification
    L0Ungrounded,
    /// L1: Formally anchored to a cryptographic passage in a verified source
    L1Cited,
    /// L2: Empirically verified via reproducible computation or invariant check
    L2Verified,
    /// L3: Invariant proof sealed into CAS Ledger (cas://bafy...)
    L3Sealed,
}

impl Default for ClaimGroundingLevel {
    fn default() -> Self {
        Self::L0Ungrounded
    }
}

/// A scientific literature, dataset, code repo, or web source record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub id: String,
    pub source_type: String, // "paper", "dataset", "web", "repo"
    pub title: String,
    pub doi: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<u32>,
    pub content_hash: String, // BLAKE3 or SHA-256
    pub local_path: Option<String>,
    pub verified: bool,
    pub abstract_text: Option<String>,
    pub created_at: i64,
}

/// A cryptographically anchored passage inside a source record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassageAnchor {
    pub id: String,
    pub source_id: String,
    pub source_title: Option<String>,
    pub section_title: Option<String>,
    pub page_number: Option<u32>,
    pub start_offset: usize,
    pub end_offset: usize,
    pub exact_text: String,
    pub passage_hash: String, // BLAKE3 hash of exact_text
}

/// Untrusted passage selection. The backend must resolve it against the stored
/// source version and compute `passage_hash` before creating a `PassageAnchor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassageAnchorProposal {
    pub source_id: String,
    pub section_title: Option<String>,
    pub page_number: Option<u32>,
    pub start_offset: usize,
    pub end_offset: usize,
    pub exact_text: String,
}

impl PassageAnchorProposal {
    pub fn new(
        source_id: impl Into<String>,
        start_offset: usize,
        end_offset: usize,
        exact_text: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let source_id = source_id.into();
        let exact_text = exact_text.into();
        if source_id.trim().is_empty() || exact_text.trim().is_empty() || start_offset >= end_offset
        {
            return Err(DomainError::Validation(
                "Passage proposal requires source, non-empty text, and a valid offset range".into(),
            ));
        }
        Ok(Self {
            source_id,
            section_title: None,
            page_number: None,
            start_offset,
            end_offset,
            exact_text,
        })
    }
}

/// Semantic relationship of evidence to a claim
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRelation {
    Supports,
    Refutes,
    Qualifies,
}

/// A link between a Claim and an anchored Passage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEvidenceLink {
    pub passage_anchor_id: String,
    pub source_title: Option<String>,
    pub exact_text: Option<String>,
    pub relation: EvidenceRelation,
    pub rationale: String,
    pub verified_by: String, // "deterministic_engine", "expert_review"
}

/// Atomic scientific proposition with formal grounding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchClaim {
    pub id: String,
    pub statement: String,
    pub level: ClaimGroundingLevel,
    pub confidence_score: f32, // 0.0 .. 1.0
    pub invariants: Vec<String>,
    pub evidence_links: Vec<ClaimEvidenceLink>,
    pub created_at: i64,
    pub sealed_proof_uri: Option<String>, // "cas://bafy2bzace..."
}

/// Untrusted client proposal. Grounding level, confidence, verifier identity,
/// content hashes, and sealed proof references are intentionally absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchClaimProposal {
    pub id: String,
    pub statement: String,
    pub invariants: Vec<String>,
    pub created_at: i64,
}

impl ResearchClaimProposal {
    pub fn new(statement: impl Into<String>, created_at: i64) -> Result<Self, DomainError> {
        let statement = statement.into();
        if statement.trim().is_empty() {
            return Err(DomainError::Validation(
                "Research claim proposal statement cannot be empty".into(),
            ));
        }
        Ok(Self {
            id: new_id("claim_proposal"),
            statement,
            invariants: Vec::new(),
            created_at,
        })
    }

    /// Materializes the proposal only at the ungrounded level. Backend verifiers
    /// must create separate verification records before status can be elevated.
    pub fn into_unverified_claim(self) -> ResearchClaim {
        ResearchClaim {
            id: self.id,
            statement: self.statement,
            level: ClaimGroundingLevel::L0Ungrounded,
            confidence_score: 0.0,
            invariants: self.invariants,
            evidence_links: Vec::new(),
            created_at: self.created_at,
            sealed_proof_uri: None,
        }
    }
}

/// Source metadata proposed by a client before backend import and hashing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceProposal {
    pub id: String,
    pub source_type: String,
    pub title: String,
    pub locator: String,
    pub doi: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<u32>,
    pub abstract_text: Option<String>,
    pub created_at: i64,
}

impl SourceProposal {
    pub fn new(
        source_type: impl Into<String>,
        title: impl Into<String>,
        locator: impl Into<String>,
        created_at: i64,
    ) -> Result<Self, DomainError> {
        let source_type = source_type.into();
        let title = title.into();
        let locator = locator.into();
        if source_type.trim().is_empty() || title.trim().is_empty() || locator.trim().is_empty() {
            return Err(DomainError::Validation(
                "Source proposal type, title, and locator are required".into(),
            ));
        }
        Ok(Self {
            id: new_id("source_proposal"),
            source_type,
            title,
            locator,
            doi: None,
            authors: Vec::new(),
            year: None,
            abstract_text: None,
            created_at,
        })
    }
}

/// Snapshot of computation environment for reproducibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvSnapshot {
    pub python_version: String,
    pub lockfile_hash: String,
    pub package_count: Option<u32>,
    pub hardware: String,
    pub platform: Option<String>,
}

/// Experiment run execution receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchExperimentRun {
    pub run_id: String,
    pub session_id: String,
    pub command: String,
    pub cwd: String,
    pub status: String, // "pending", "running", "ok", "failed"
    pub wall_ms: u64,
    pub surface: Option<String>, // "local", "hpc", "modal", "jupyter", "sandbox"
    pub reproducibility: String, // "deterministic", "partial", "unverified"
    pub input_merkle_root: String,
    pub output_merkle_root: String,
    pub env_snapshot: EnvSnapshot,
    pub sade_permit_id: Option<String>,
    pub cas_log_uri: Option<String>,
    pub ts: i64,
}

/// Content-addressed lineage node of generated artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactLineageNode {
    pub artifact_path: String,
    pub version: u32,
    pub content_hash: String,
    pub produced_by_run_id: Option<String>,
    pub parent_version_hash: Option<String>,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_claim_proposal_materializes_without_trusted_status() {
        let proposal = ResearchClaimProposal::new("The selected method improves recall", 1);
        assert!(proposal.is_ok());
        if let Ok(value) = proposal {
            let claim = value.into_unverified_claim();
            assert_eq!(claim.level, ClaimGroundingLevel::L0Ungrounded);
            assert_eq!(claim.confidence_score, 0.0);
            assert!(claim.evidence_links.is_empty());
            assert!(claim.sealed_proof_uri.is_none());
        }
    }

    #[test]
    fn passage_proposal_cannot_supply_a_trusted_hash() {
        let proposal = PassageAnchorProposal::new("source_1", 10, 20, "selected passage");
        assert!(proposal.is_ok());
        if let Ok(value) = proposal {
            assert_eq!(value.source_id, "source_1");
            assert_eq!(value.start_offset, 10);
            assert_eq!(value.end_offset, 20);
        }
    }
}
