//! Research Block (Core Layer)
//!
//! Contracts, draft ingress, and verification pipelines empowering the Research Workbench:
//! notebook kernel interactions, experiment lifecycle, checkpoint storage,
//! research draft ingress boundaries, and cryptographic evidence verification pipelines.

pub use crate::contracts::checkpoint_store::{
    BlobMetadata, CheckpointStorePort, PromotionOutcome, StoreCapabilities,
};
pub use crate::contracts::experiment::{
    AttemptObservation, ExecutionTargetPort, PreflightReport, TargetCapabilities,
};
pub use crate::contracts::notebook::{
    NotebookDriverPort, NotebookWorkerResponse, NotebookWorkerSession,
};
pub use crate::evidence::{
    CitationVerifier, CommandExitCodeVerifier, DiffVerifier, EvidenceBundle, EvidencePipeline,
    ExactMatchVerifier, HashVerifier, PatchPreviewVerifier, RequirementEvaluation,
    SemanticSupportEvaluator,
};
pub use crate::research_ingress::{
    claim_draft, experiment_draft, passage_draft, source_draft, source_draft_with_bytes,
};
