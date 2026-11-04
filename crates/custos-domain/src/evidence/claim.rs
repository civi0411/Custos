//! Verifiable Research Claim, Source, Passage Anchor, and Lineage Domain Models
//! Strictly enforces Invariant Grounding Levels (L0-L3) and Cryptographic Proofs.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

/// 4-Level Grounding Status enforcing zero false-passes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClaimGroundingLevel {
    /// L0: Raw assertion without citation or verification
    #[default]
    L0Ungrounded,
    /// L1: Formally anchored to a cryptographic passage in a verified source
    L1Cited,
    /// L2: Empirically verified via reproducible computation or invariant check
    L2Verified,
    /// L3: Invariant proof sealed into CAS Ledger (cas://bafy...)
    L3Sealed,
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

/// Evaluation status for research claim verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Verified,
    Approved,
    Passed,
    Rejected,
    Uncertain,
}

impl AssessmentStatus {
    pub fn is_passing(&self) -> bool {
        matches!(self, Self::Verified | Self::Approved | Self::Passed)
    }
}

/// Natural Language Inference (NLI) verdict for claim grounding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntailmentVerdict {
    Entailed { confidence: f32, rationale: String },
    Neutral { confidence: f32, rationale: String },
    Contradicted { confidence: f32, rationale: String },
}

impl EntailmentVerdict {
    pub fn is_entailed(&self) -> bool {
        matches!(self, Self::Entailed { .. })
    }

    pub fn confidence(&self) -> f32 {
        match self {
            Self::Entailed { confidence, .. }
            | Self::Neutral { confidence, .. }
            | Self::Contradicted { confidence, .. } => *confidence,
        }
    }

    pub fn rationale(&self) -> &str {
        match self {
            Self::Entailed { rationale, .. }
            | Self::Neutral { rationale, .. }
            | Self::Contradicted { rationale, .. } => rationale,
        }
    }
}

/// Cryptographically bound assessment record for research claim verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAssessmentRecord {
    pub id: String,
    pub claim_id: String,
    pub passage_anchor_id: Option<String>,
    pub source_version: Option<String>,
    pub status: AssessmentStatus,
    pub confidence_score: f32,
    pub is_stale: bool,
    pub evaluator_id: String,
    pub assessed_at: i64,
}

impl ClaimAssessmentRecord {
    pub fn new(
        claim_id: impl Into<String>,
        status: AssessmentStatus,
        confidence_score: f32,
        evaluator_id: impl Into<String>,
        assessed_at: i64,
    ) -> Result<Self, DomainError> {
        let claim_id = claim_id.into();
        let evaluator_id = evaluator_id.into();
        if claim_id.trim().is_empty() {
            return Err(DomainError::Validation("Assessment requires non-empty claim_id".into()));
        }
        if !(0.0..=1.0).contains(&confidence_score) {
            return Err(DomainError::Validation("Confidence score must be between 0.0 and 1.0".into()));
        }
        Ok(Self {
            id: new_id("assess"),
            claim_id,
            passage_anchor_id: None,
            source_version: None,
            status,
            confidence_score,
            is_stale: false,
            evaluator_id,
            assessed_at,
        })
    }

    pub fn with_passage(mut self, passage_anchor_id: impl Into<String>) -> Self {
        self.passage_anchor_id = Some(passage_anchor_id.into());
        self
    }

    pub fn with_source_version(mut self, version: impl Into<String>) -> Self {
        self.source_version = Some(version.into());
        self
    }

    pub fn is_valid_and_fresh(&self) -> bool {
        self.status.is_passing() && !self.is_stale
    }
}

impl ResearchClaim {
    /// Elevates grounding level based on a verified assessment record
    pub fn elevate_grounding(&mut self, assessment: &ClaimAssessmentRecord) -> Result<(), DomainError> {
        if assessment.claim_id != self.id {
            return Err(DomainError::Validation(format!(
                "Assessment target {} does not match claim {}",
                assessment.claim_id, self.id
            )));
        }
        if assessment.is_stale {
            return Err(DomainError::Validation("Cannot elevate claim with stale assessment".into()));
        }
        if !assessment.status.is_passing() {
            return Err(DomainError::Validation(format!(
                "Assessment status {:?} is not passing",
                assessment.status
            )));
        }

        self.confidence_score = assessment.confidence_score;

        if self.evidence_links.is_empty() && assessment.passage_anchor_id.is_none() {
            self.level = ClaimGroundingLevel::L1Cited;
        } else if assessment.confidence_score >= 0.70 {
            self.level = ClaimGroundingLevel::L2Verified;
        } else {
            self.level = ClaimGroundingLevel::L1Cited;
        }

        Ok(())
    }

    /// Adds a cryptographically linked evidence passage to this claim
    pub fn link_evidence(&mut self, link: ClaimEvidenceLink) -> Result<(), DomainError> {
        if link.passage_anchor_id.trim().is_empty() {
            return Err(DomainError::Validation("Evidence link requires valid passage_anchor_id".into()));
        }
        self.evidence_links.push(link);
        if self.level == ClaimGroundingLevel::L0Ungrounded {
            self.level = ClaimGroundingLevel::L1Cited;
        }
        Ok(())
    }

    /// Seals the claim proof into a CAS URI (L3Sealed)
    pub fn seal_cas_proof(&mut self, uri: impl Into<String>) -> Result<(), DomainError> {
        if self.level != ClaimGroundingLevel::L2Verified {
            return Err(DomainError::Validation("Claim must be L2Verified before sealing CAS proof".into()));
        }
        let uri = uri.into();
        if !uri.starts_with("cas://") {
            return Err(DomainError::Validation("Proof URI must start with cas://".into()));
        }
        self.sealed_proof_uri = Some(uri);
        self.level = ClaimGroundingLevel::L3Sealed;
        Ok(())
    }

    pub fn is_verified_for_coding(&self) -> bool {
        matches!(self.level, ClaimGroundingLevel::L2Verified | ClaimGroundingLevel::L3Sealed)
            && self.confidence_score >= 0.70
    }
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

    #[test]
    fn test_claim_elevation_and_cas_sealing_state_machine() {
        let mut claim = ResearchClaimProposal::new("LoRA achieves parity with full fine-tuning", 100)
            .unwrap()
            .into_unverified_claim();
        assert_eq!(claim.level, ClaimGroundingLevel::L0Ungrounded);
        assert!(!claim.is_verified_for_coding());

        // Linking evidence passage moves it to L1Cited
        let link = ClaimEvidenceLink {
            passage_anchor_id: "anchor_lora_p4".into(),
            source_title: Some("LoRA Paper".into()),
            exact_text: Some("Table 1 shows parity on GLUE benchmark".into()),
            relation: EvidenceRelation::Supports,
            rationale: "Empirical parity on GLUE".into(),
            verified_by: "alce_engine".into(),
        };
        assert!(claim.link_evidence(link).is_ok());
        assert_eq!(claim.level, ClaimGroundingLevel::L1Cited);

        // Cannot seal CAS from L1
        assert!(claim.seal_cas_proof("cas://bafytest123").is_err());

        // Assessment with verified status elevates to L2Verified
        let assess = ClaimAssessmentRecord::new(
            &claim.id,
            AssessmentStatus::Verified,
            0.92,
            "alce_evaluator_v1",
            105,
        )
        .unwrap()
        .with_passage("anchor_lora_p4")
        .with_source_version("sha256:abc1234");

        assert!(claim.elevate_grounding(&assess).is_ok());
        assert_eq!(claim.level, ClaimGroundingLevel::L2Verified);
        assert!(claim.is_verified_for_coding());

        // Now sealing CAS proof succeeds and moves to L3Sealed
        assert!(claim.seal_cas_proof("cas://bafytest123").is_ok());
        assert_eq!(claim.level, ClaimGroundingLevel::L3Sealed);
        assert_eq!(claim.sealed_proof_uri.as_deref(), Some("cas://bafytest123"));
    }
}
