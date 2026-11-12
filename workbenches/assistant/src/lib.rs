//! Custos Assistant Domain Pack
//!
//! Authoritative mapping to Custos SADE Master Specification Part 12.
//! Covers personal productivity, inbox triage, safe draft generation, calendar scheduling,
//! and governed external mutations across the Copilot / Assistant Workbench Lens.

pub mod domain;
pub mod fixtures;
pub mod profile;
pub mod repository;
pub mod skills;
pub mod supervision;
pub mod verifiers;

pub use domain::{AssistantDraft, AssistantDraftPort, GovernedOutboxInvariant};
pub use fixtures::*;
pub use profile::*;
pub use repository::*;
pub use skills::*;
pub use verifiers::*;

use async_trait::async_trait;
use custos_domain::oi::ExecutionTopology;
use custos_domain::session::WorkbenchLens;
use custos_domain::DomainError;
use std::sync::Arc;

use custos_core::contracts::pack::{
    DomainPack, DomainPackManifest, PackProfile, PackVerifier, WorkbenchJob, WorkbenchModule,
    WorkerRole,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantPackDescriptor {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

pub fn get_descriptor() -> AssistantPackDescriptor {
    AssistantPackDescriptor {
        name: "custos-pack-assistant".to_string(),
        version: "0.1.0".to_string(),
        features: vec![
            "Workspace Inbox Triage & Summarization".to_string(),
            "Safe Draft Reply Generation (Zero Phantom Send)".to_string(),
            "Meeting & Calendar Action Item Scheduling".to_string(),
            "Governed Outbox Execution with User Acceptance".to_string(),
            "Recurring Automation with Revocable Grants".to_string(),
        ],
    }
}

pub struct AssistantPack;

impl AssistantPack {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AssistantPack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Clone)]
pub struct AssistantModule;

impl AssistantModule {
    pub fn new() -> Self {
        Self
    }
}

impl WorkbenchModule for AssistantModule {
    fn id(&self) -> &'static str {
        "assistant"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["copilot"]
    }

    fn pack(&self) -> Arc<dyn DomainPack> {
        Arc::new(AssistantPack::new())
    }

    fn register_skills(&self, registry: &mut dyn custos_core::contracts::skill::SkillSink) {
        skills::register_assistant_skills(registry);
    }

    fn migrations(&self) -> &'static [&'static str] {
        &[include_str!("../migrations/assistant_schema.sql")]
    }
}


#[async_trait]
impl DomainPack for AssistantPack {
    fn manifest(&self) -> DomainPackManifest {
        let roles = self.roles().into_iter().map(|r| r.role_id).collect();
        let jobs = self.jobs().into_iter().map(|j| j.job_id).collect();
        DomainPackManifest {
            id: "assistant".to_string(),
            name: "Assistant Pack".to_string(),
            version: "0.1.0".to_string(),
            lens: WorkbenchLens::Assistant,
            description: "Personal task assistance, inbox triage, draft generation, and governed side-effects".to_string(),
            supported_jobs: jobs,
            supported_roles: roles,
            required_capabilities: vec![],
        }
    }

    fn profile(&self) -> PackProfile {
        profile::assistant_profile()
    }

    fn roles(&self) -> Vec<WorkerRole> {
        vec![
            WorkerRole::new(
                "triage_worker",
                "assistant",
                "Triage Worker",
                "Inbox and notification priority classifier that extracts action items without mutating email state",
                "You are an executive triage assistant. Analyze incoming emails, messages, and task feeds. Summarize key requests, detect urgency tiers, and highlight required decisions without deleting or sending messages.",
                5_000,
                &["contact_resolve", "file_read"],
                true,
            ),
            WorkerRole::new(
                "draft_worker",
                "assistant",
                "Draft Worker",
                "Safe communication composer creating draft responses and delegation proposals with locked payloads",
                "You are a careful communications drafter. Formulate clear, concise email drafts and meeting invites. You are strictly restricted to draft generation; you never initiate network transmission.",
                8_000,
                &["payload_lock", "contact_resolve", "file_read"],
                false, // Can write draft entities locally
            ),
            WorkerRole::new(
                "schedule_worker",
                "assistant",
                "Schedule Worker",
                "Calendar availability optimizer that resolves IANA timezones and daylight savings constraints",
                "You are a calendar and schedule coordinator. Inspect availability slots across multiple participants, resolve timezones correctly, and propose non-conflicting meeting proposals.",
                5_000,
                &["contact_resolve", "file_read"],
                true,
            ),
            WorkerRole::new(
                "outbox_dispatcher",
                "assistant",
                "Outbox Dispatcher",
                "Governed effect executor that dispatches approved payloads via connector and logs immutable receipts",
                "You are a transactional outbox dispatcher. Verify that user approval has been granted and that payload hashes match exactly before dispatching external side-effects.",
                5_000,
                &["payload_lock"],
                false,
            ),
        ]
    }

    fn jobs(&self) -> Vec<WorkbenchJob> {
        vec![
            WorkbenchJob::new(
                "assistant__personal_qa",
                WorkbenchLens::Assistant,
                "Personal QA & Briefing",
                "Answer user questions from personal notes, tasks, and recent activity with temporal freshness",
                &["query"],
                &["BriefingAnswer", "TemporalSourceRefs"],
                ExecutionTopology::T1SingleWorker,
                "triage_worker",
                &[],
                "Accurate answer grounded in recent personal history with identity and privacy boundaries preserved",
            ),
            WorkbenchJob::new(
                "assistant__inbox_triage",
                WorkbenchLens::Assistant,
                "Inbox Triage & Draft",
                "Classify unread messages, prioritize urgent requests, and formulate candidate draft replies",
                &["inbox_source", "priority_filter"],
                &["TriageSummary", "DraftMessages"],
                ExecutionTopology::T2StagedPipeline,
                "triage_worker",
                &[],
                "High quality draft produced in Draft status; zero un-permitted external side effects",
            ),
            WorkbenchJob::new(
                "assistant__schedule_proposal",
                WorkbenchLens::Assistant,
                "Meeting & Agenda Schedule Proposal",
                "Coordinate meeting time slots respecting IANA timezones, availability, and preferences",
                &["attendees", "duration_minutes", "timeframe"],
                &["CandidateSlots", "CalendarEventDraft"],
                ExecutionTopology::T1SingleWorker,
                "schedule_worker",
                &[],
                "Zero calendar overlaps, valid IANA timezones with daylight savings checks",
            ),
            WorkbenchJob::new(
                "assistant__mutate_execution",
                WorkbenchLens::Assistant,
                "Governed Outbox Execution",
                "Dispatch approved email, event creation, or message with cryptographic approval verification",
                &["approved_payload", "recipient", "approval_id"],
                &["OutboxReceipt", "AuditLogEntry"],
                ExecutionTopology::T1SingleWorker,
                "outbox_dispatcher",
                &["user_acceptance_oracle"],
                "Verified user approval, untampered payload digest matching preview, connector receipt confirmed",
            ),
            WorkbenchJob::new(
                "assistant__reminder_automation",
                WorkbenchLens::Assistant,
                "Recurring Reminder & Automation",
                "Set up recurring scheduled check or reminder within revocable grant boundaries",
                &["cron_schedule", "target_action", "max_runs"],
                &["AutomationRegistration", "RevocableGrantSpec"],
                ExecutionTopology::T1SingleWorker,
                "triage_worker",
                &["user_acceptance_oracle"],
                "Clear trigger frequency, explicit expiration deadline, bounded per-run token limits",
            ),
        ]
    }

    fn verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
        vec![Arc::new(verifiers::UserAcceptanceOracle::new())]
    }

    async fn initialize(&self) -> Result<(), DomainError> {
        tracing::info!(
            "Initialized Assistant Domain Pack with Triage, Agenda, and Acceptance capabilities"
        );
        Ok(())
    }

    fn lane_policy(&self) -> custos_domain::LanePolicy {
        custos_domain::LanePolicy::interactive_focused()
    }

    fn supervision_profile(&self) -> Option<custos_domain::supervision::SupervisionProfile> {
        Some(supervision::assistant_supervision_profile())
    }

    fn tool_declarations(&self) -> Vec<serde_json::Value> {
        vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "schedule_task",
                "description": "Propose or schedule an automated assistant task or reminder",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Task or reminder title" },
                        "due_at": { "type": "string", "description": "ISO timestamp for execution" }
                    },
                    "required": ["title"]
                }
            }
        })]
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        match tool_name {
            "schedule_task" => {
                let title = params
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled Task");
                let due_at = params.get("due_at").and_then(|v| v.as_str());
                Ok(serde_json::json!({
                    "status": "scheduled",
                    "title": title,
                    "due_at": due_at,
                    "scheduled_at": chrono::Utc::now().to_rfc3339(),
                }))
            }
            _ => Err(DomainError::Validation(format!(
                "Unsupported assistant tool: '{tool_name}'"
            ))),
        }
    }
}
