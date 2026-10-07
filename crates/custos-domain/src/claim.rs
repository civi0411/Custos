//! Verifiable Research Claim, Source, Passage Anchor, and Lineage Domain Models
//! Strictly enforces Invariant Grounding Levels (L0-L3) and Cryptographic Proofs.

use serde::{Deserialize, Serialize};

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
