//! Copilot Workbench Runtime Block
//!
//! Provides the primary cognitive agent runtime: System 1 / System 2 routing,
//! interactive chat session management, prompt synthesis, and omni-intelligence replanning.

pub use crate::agent::runtime_port::{
    GovernedAgentRuntime, GovernedToolExecutor, InvocationContext, LimitedWriter, ToolResult,
};
pub use crate::cognitive::approval_router::{
    ApprovalDecision, ApprovalKind, ApprovalPolicy, ApprovalRouter,
};
pub use crate::cognitive::arbiter::CognitiveArbiter;
pub use crate::cognitive::config::CognitiveConfig;
pub use crate::cognitive::deliberation::{
    DeliberationPlan, DeliberationResult, RoleCritique, SystemTwoOrchestrator, WorkerRole,
};
pub use crate::cognitive::human_gate::{
    HumanGate, HumanGateOutcome, HumanGateRequest, PendingClaim,
};
pub use crate::cognitive::pipeline::{
    A2ADelegationPort, CognitiveExecutionResult, CognitivePipeline,
};
pub use crate::cognitive::prompt_manager::{
    CustosAgentMode, ExtensionPromptInfo, PromptManager, SystemPromptBuilder,
};
pub use crate::cognitive::provider_selector::{
    ProviderPreference, ProviderSelector, TierProviderMap,
};
pub use crate::cognitive::registry::{HealthStatus, ProviderHealth, ProviderRegistry};
pub use crate::cognitive::routing::{
    A2ATarget, ContextMode, DelegationMode, FallbackRoute, ReasoningTier, RouteDecision,
    RoutingPolicy, RoutingSignals,
};
pub use crate::context::compaction as context_management;
pub use crate::context::memory as memory_service;
pub use crate::oi::engine::OiEngine;
pub use crate::session::journal::SessionJournal;
pub use crate::session::manager::SessionManager;
