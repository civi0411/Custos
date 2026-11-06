//! Copilot & Assistant Block (Domain Layer)
//!
//! Provides the core domain aggregates and value objects for the Copilot Workbench:
//! agent conversations, assistant drafts, outbox action items, cognitive contexts,
//! memory entries, authority grants, permits, and human-in-the-loop approvals.

pub use crate::authority::action::{
    Action, ActionIntent, ActionLifecycleState, Assurance, EffectAttempt, EffectStatus, RiskLevel,
};
pub use crate::authority::approval::{ApprovalDecision, ApprovalRequest, ApprovalStatus};
pub use crate::authority::capability::{
    CapabilityDescriptor, CapabilityGroup, CapabilityManifest, CapabilityStatus,
};
pub use crate::authority::permit::{
    ExecutionPermit, ExecutionReceipt, Grant, Permit, PermitId, Receipt, ReceiptStatus, RiskClass,
};
pub use crate::context::annotation::{AnnotationRecord, AnnotationStatus, AnnotationTarget};
pub use crate::context::context::{
    ContextDeliveryState, ContextItem, ContextPack, ContextReceipt, OmittedContextReason,
    OmittedContextRef,
};
pub use crate::context::memory::{
    FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact, ProposalReceipt,
    ProposalStatus, RecallQuery, WorkerRunId,
};
pub use crate::session::assistant::{
    AssistantDraft, OutboxActionItem, OutboxDeliveryState, ResourceActionCapability, StandingGrant,
};
pub use crate::session::automation::{
    CreateHeadlessJobParams, HeadlessAutomationJob, HeadlessJobStatus, HeadlessTaskSpec,
    HeadlessTrigger, RunHeadlessJobResult,
};
pub use crate::session::session::{
    ConversationTurn, Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus,
    SessionTaskBinding, TurnActorKind, WorkbenchLens,
};
