//! Copilot & Assistant Block (Core Layer)
//!
//! State machines, authority governance, and kernel contracts empowering the Copilot Workbench:
//! turn execution state machine, commands/events, approval coordination, grant evaluation,
//! permit issuance, and external harness contracts.

pub use crate::authority::{
    ApprovalManager, AuditEntry, AuditLog, AuthorityEngine, DefaultPolicyEvaluator, GrantStore,
    PermitIssuer, PolicyDecision, PolicyEvaluator, RiskEvaluator,
};
pub use crate::contracts::harness::{
    AgentRuntimePort, CostVisibility, HarnessExecutionResult, HarnessProfile, ToolMediationLevel,
    WorktreeOwnership,
};
pub use crate::contracts::kernel::{KernelPort, TrustedKernel};
pub use crate::kernel::{
    AdvanceTask, BudgetGovernor, CancelTask, CompleteTask, CompletionGate, CreateTask,
    KernelObservability, RetrySpec, SessionStateMachine, TaskEvent, TaskInvariants, TaskReducer,
    TaskService, TaskStateMachine,
};
