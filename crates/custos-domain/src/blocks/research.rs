//! Research Block (Domain Layer)
//!
//! Provides the core domain aggregates and value objects for the Research Workbench:
//! interactive Jupyter notebooks, evidence chains, claim citations, scientific reviews,
//! synthesis handoffs, and deterministic experiment attempts.

pub use crate::evidence::claim::{
    ArtifactLineageNode, AssessmentStatus, ClaimAssessmentRecord, ClaimEvidenceLink,
    ClaimGroundingLevel, EntailmentVerdict, EnvSnapshot, EvidenceRelation, PassageAnchor,
    PassageAnchorProposal, ResearchClaim, ResearchClaimProposal, ResearchExperimentRun,
    SourceProposal, SourceRecord,
};
pub use crate::evidence::evidence::{
    CriterionVerificationRecord, EvidenceRecord, EvidenceRecordV1, EvidenceRequirement,
    EvidenceStatus, OutcomeStatus, TaskOutcome, VerificationClaim,
};
pub use crate::evidence::execution_record::{
    ExecutionArtifact, ExecutionRecord, ExecutionRecordStatus,
};
pub use crate::evidence::experiment_attempt::{
    Check, CheckGroup, CheckStatus, ExecEvent, ExecState, ExecutionAttempt, ExternalJobRef,
    Invocation, InvocationOutcome, ProgressTriplet, ProtectionPolicy, ProtectionStatus,
    ResourceEvent, ResourceState, ResumeVerdict, TargetCaps, VerificationReport,
};
pub use crate::evidence::fact::Fact;
pub use crate::evidence::review::{
    FindingSeverity, RecordReviewParams, ReviewFinding, ReviewMethod, ReviewStatus,
    ReviewTargetType, ReviewerRecord,
};
pub use crate::evidence::synthesis::{
    ClaimHandoffSummary, HandoffToCodingParams, HandoffToCodingResult, RecipeHandoffSummary,
    ResearchSynthesisProposal, SaveSynthesisProposalParams, SynthesisProposalStatus,
};
pub use crate::research::experiment::{
    FrozenTrainingRecipe, ModelEvaluationReport, ModelEvaluationSpec, TrainingExecutionReceipt,
    TrainingStepTelemetry,
};
pub use crate::research::note::{NoteRecord, NoteVersionRecord};
pub use crate::research::notebook::{
    CellExecutionStatus, ExecuteCellParams, ExecuteCellResult, KernelStatus, NotebookCell,
    NotebookCellType, NotebookDocument, NotebookKernelBinding, NotebookKernelState,
};
