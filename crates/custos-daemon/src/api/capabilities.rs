//! Kernel Capabilities Catalog Inspection for Custos ADE

use custos_domain::{CapabilityDescriptor, CapabilityGroup};

use super::LocalApiDispatcher;
use super::transport::{browser_execution_adapter_connected, remote_execution_adapter_connected};

impl LocalApiDispatcher {
    /// List all capabilities registered in the Custos ADE kernel truthfully.
    ///
    /// Workbenches project views over these capabilities; they do not maintain
    /// simulated independent backends. Unimplemented or unconfigured surfaces
    /// report `Unavailable` or `Degraded` with an explicit reason.
    pub fn list_capabilities(&self) -> Vec<CapabilityDescriptor> {
        vec![
            // Core Task Engine
            CapabilityDescriptor::available(
                "tasks.core",
                "Task Engine",
                CapabilityGroup::Coordination,
                "Kernel task lifecycle, contract enforcement, and verification proofs.",
                Some("tasks"),
                vec!["create", "get", "cancel", "advance", "complete"],
            ),
            // Workspaces (Git & Folder Execution Workspaces)
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "workspace.git",
                    "Workspaces",
                    CapabilityGroup::Code,
                    "Inspect daemon-owned folder and Git execution workspaces.",
                    Some("worktrees"),
                    vec!["create", "list", "get", "archive", "inspect_dirty", "recover"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "workspace.git",
                    "Workspaces",
                    CapabilityGroup::Code,
                    "Inspect daemon-owned folder and Git execution workspaces.",
                    Some("worktrees"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
            // Workflow DAG Engine
            if self.workflow.is_some() {
                CapabilityDescriptor::available(
                    "workflow.dag",
                    "Workflow",
                    CapabilityGroup::Coordination,
                    "Execution graph, dependencies, budgets, and run orchestration.",
                    Some("dag"),
                    vec!["start_run", "cancel_run"],
                )
            } else {
                CapabilityDescriptor::degraded(
                    "workflow.dag",
                    "Workflow",
                    CapabilityGroup::Coordination,
                    "Execution graph, dependencies, budgets, and run orchestration.",
                    Some("dag"),
                    "Workflow engine is not attached; graph orchestration is degraded.",
                    vec![],
                )
            },
            // Research Literature Sources
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.sources",
                    "Sources",
                    CapabilityGroup::Evidence,
                    "Versioned literature, passages, DOI anchors, and corpus coverage.",
                    Some("literature"),
                    vec!["list", "save"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.sources",
                    "Sources",
                    CapabilityGroup::Evidence,
                    "Versioned literature, passages, DOI anchors, and corpus coverage.",
                    Some("literature"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Claims Matrix
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.claims",
                    "Claims",
                    CapabilityGroup::Evidence,
                    "Atomic claims with support, contradiction, or unknown evidence grounding.",
                    Some("claims"),
                    vec!["list", "save", "handoff_coding"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.claims",
                    "Claims",
                    CapabilityGroup::Evidence,
                    "Atomic claims with support, contradiction, or unknown evidence grounding.",
                    Some("claims"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Methods / Recipes & Executions
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.methods",
                    "Methods",
                    CapabilityGroup::Evidence,
                    "Reproduction recipes separated from observed execution records.",
                    Some("knowledge"),
                    vec!["recipes.list", "recipes.get", "executions.list", "executions.get", "executions.resume.assess"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.methods",
                    "Methods",
                    CapabilityGroup::Evidence,
                    "Reproduction recipes separated from observed execution records.",
                    Some("knowledge"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Runs Ledger
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.runs",
                    "Runs",
                    CapabilityGroup::Compute,
                    "Experiment attempts, environments, outputs, and execution receipts.",
                    Some("runs"),
                    vec!["list", "save"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.runs",
                    "Runs",
                    CapabilityGroup::Compute,
                    "Experiment attempts, environments, outputs, and execution receipts.",
                    Some("runs"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Research Artifacts & Lineage
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "research.artifacts",
                    "Artifacts",
                    CapabilityGroup::Evidence,
                    "Version lineage, annotations, provenance DAG, and review findings.",
                    Some("artifacts"),
                    vec!["list", "get", "record_lineage", "lineage.list", "lineage.graph", "annotations.list", "annotations.save"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "research.artifacts",
                    "Artifacts",
                    CapabilityGroup::Evidence,
                    "Version lineage, annotations, provenance DAG, and review findings.",
                    Some("artifacts"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            // Providers (LLM backend)
            if self.providers.is_some() {
                CapabilityDescriptor::available(
                    "providers.llm",
                    "Providers",
                    CapabilityGroup::Conversation,
                    "Model provider credentials, client keys, and health probes.",
                    Some("providers"),
                    vec!["list", "save", "check_status", "keys.list", "keys.generate"],
                )
            } else {
                CapabilityDescriptor::degraded(
                    "providers.llm",
                    "Providers",
                    CapabilityGroup::Conversation,
                    "Model provider credentials, client keys, and health probes.",
                    Some("providers"),
                    "Provider repository not configured on daemon instance.",
                    vec![],
                )
            },
            // Terminal PTY (Native PTY stream scoped to execution workspace)
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "compute.pty",
                    "Terminal",
                    CapabilityGroup::Compute,
                    "Bounded PTY streams scoped to an execution workspace.",
                    Some("terminal"),
                    vec!["spawn", "write", "resize", "read", "terminate", "list", "get"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "compute.pty", "Terminal", CapabilityGroup::Compute,
                    "Bounded PTY streams scoped to an execution workspace.", Some("terminal"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "code.files", "Files", CapabilityGroup::Code,
                    "Repository tree, file buffers, and source anchor inspection.", Some("files"),
                    vec!["tree", "read", "write"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "code.files", "Files", CapabilityGroup::Code,
                    "Repository tree, file buffers, and source anchor inspection.", Some("files"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
            if self.workspace.is_some() {
                CapabilityDescriptor::available(
                    "code.changes", "Changes", CapabilityGroup::Code,
                    "Diff review and Git stage decisions in an execution workspace.", Some("changes"),
                    vec!["diff", "file_diff", "stage", "unstage", "discard"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "code.changes", "Changes", CapabilityGroup::Code,
                    "Diff review and Git stage decisions in an execution workspace.", Some("changes"),
                    "WorkspaceCoordinator is not configured on this daemon instance.",
                )
            },
            if self.python_kernel.is_some() {
                CapabilityDescriptor::available(
                    "compute.notebook",
                    "Notebook",
                    CapabilityGroup::Compute,
                    "Authorized kernels, code cells, and reproducible compute epochs.",
                    Some("experiments"),
                    vec!["cells.list", "cells.save", "execute", "interrupt", "reset", "status"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "compute.notebook",
                    "Notebook",
                    CapabilityGroup::Compute,
                    "Authorized kernels, code cells, and reproducible compute epochs.",
                    Some("experiments"),
                    "PythonKernelCoordinator is not configured on this daemon instance.",
                )
            },
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "evidence.criteria",
                    "Evidence",
                    CapabilityGroup::Evidence,
                    "Criteria, receipts, verifier records, and freshness status.",
                    Some("evidence"),
                    vec!["reviews.list", "reviews.record", "reviews.get", "reviews.mark_stale"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "evidence.criteria",
                    "Evidence",
                    CapabilityGroup::Evidence,
                    "Criteria, receipts, verifier records, and freshness status.",
                    Some("evidence"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            if self.workflow.is_some() && self.usage_runs.is_some() {
                CapabilityDescriptor::available(
                    "coordination.kanban", "Activity", CapabilityGroup::Coordination,
                    "Persisted Task runs and live run events.", Some("kanban"),
                    vec!["runs.list", "runs.watch", "runs.cancel"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "coordination.kanban", "Activity", CapabilityGroup::Coordination,
                    "Persisted Task runs and live run events.", Some("kanban"),
                    "Workflow and persisted run projection are required.",
                )
            },
            if self.harnesses.list().iter().any(|harness| harness.is_available) {
                CapabilityDescriptor::available(
                    "coding.agents",
                    "Coding Agents",
                    CapabilityGroup::Code,
                    "Start a registered native coding harness in a selected execution workspace.",
                    Some("agents"),
                    vec!["list_harnesses", "start_run", "cancel_run"],
                )
            } else {
                CapabilityDescriptor::degraded(
                    "coding.agents",
                    "Coding Agents",
                    CapabilityGroup::Code,
                    "Inspect registered native coding harnesses.",
                    Some("agents"),
                    "No registered coding harness binary is available on this machine.",
                    vec!["list_harnesses"],
                )
            },
            if self.assistant.is_some() {
                CapabilityDescriptor::available(
                    "assistant.drafts",
                    "Drafts",
                    CapabilityGroup::Personal,
                    "Durable communication drafts stored by the daemon.",
                    Some("drafts"),
                    vec!["list", "get", "save", "delete"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "assistant.drafts",
                    "Drafts",
                    CapabilityGroup::Personal,
                    "Durable communication drafts stored by the daemon.",
                    Some("drafts"),
                    "AssistantRepository is not configured on this daemon instance.",
                )
            },
            if self.outbox.is_some() {
                CapabilityDescriptor::available(
                    "assistant.outbox",
                    "Outbox",
                    CapabilityGroup::Personal,
                    "Inspect, approve, and dispatch recorded external action items and receipts.",
                    Some("outbox"),
                    vec!["list", "create", "approve", "dispatch"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "assistant.outbox",
                    "Outbox",
                    CapabilityGroup::Personal,
                    "Inspect recorded external effect attempts and receipts.",
                    Some("outbox"),
                    "OutboxRepository is not configured on this daemon instance.",
                )
            },
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "synthesis.handoff",
                    "Synthesis Handoff",
                    CapabilityGroup::Coordination,
                    "Research claim synthesis, proposal drafting, recipe conversion, and Coding handoff.",
                    Some("synthesis"),
                    vec!["proposals.list", "proposals.save", "proposals.get", "handoff.execute"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "synthesis.handoff",
                    "Synthesis Handoff",
                    CapabilityGroup::Coordination,
                    "Research claim synthesis, proposal drafting, recipe conversion, and Coding handoff.",
                    Some("synthesis"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
            if self.fleet_automation.is_some() {
                if browser_execution_adapter_connected() {
                    CapabilityDescriptor::available(
                        "browser.tabs",
                        "Browser",
                        CapabilityGroup::Browser,
                        "Governed headless browser automation via agent-browser.",
                        Some("browser"),
                        vec!["sessions.list", "sessions.create", "tabs.list", "tabs.create", "tabs.navigate", "tabs.click", "tabs.fill", "tabs.snapshot", "tabs.screenshot", "tabs.close"],
                    )
                } else {
                    CapabilityDescriptor::degraded(
                        "browser.tabs",
                        "Browser",
                        CapabilityGroup::Browser,
                        "Persisted browser session and tab intents.",
                        Some("browser"),
                        "Browser execution adapter is not connected; navigation and snapshots are refused.",
                        vec!["sessions.list", "sessions.create", "tabs.list", "tabs.create", "tabs.close"],
                    )
                }
            } else {
                CapabilityDescriptor::unavailable(
                    "browser.tabs",
                    "Browser",
                    CapabilityGroup::Browser,
                    "Scoped browsing, DOM snapshots, network inspector, and page capture.",
                    Some("browser"),
                    "FleetAutomationRepository is not configured on this daemon instance.",
                )
            },
            if self.fleet_automation.is_some() {
                if remote_execution_adapter_connected() {
                    CapabilityDescriptor::available(
                        "remote.fleet",
                        "Remote Fleet",
                        CapabilityGroup::Compute,
                        "SSH remote host inventory, ping latency probes, and distributed node execution.",
                        Some("fleet"),
                        vec!["hosts.list", "hosts.register", "hosts.ping", "fleet.exec"],
                    )
                } else {
                    CapabilityDescriptor::degraded(
                        "remote.fleet",
                        "Remote Fleet",
                        CapabilityGroup::Compute,
                        "Persisted SSH host inventory.",
                        Some("fleet"),
                        "SSH transport is not connected; probes and remote execution are refused.",
                        vec!["hosts.list", "hosts.register"],
                    )
                }
            } else {
                CapabilityDescriptor::unavailable(
                    "remote.fleet",
                    "Remote Fleet",
                    CapabilityGroup::Compute,
                    "SSH remote host inventory, ping latency probes, and distributed node execution.",
                    Some("fleet"),
                    "FleetAutomationRepository is not configured on this daemon instance.",
                )
            },
            if self.fleet_automation.is_some() {
                CapabilityDescriptor::available(
                    "automation.headless",
                    "Automation",
                    CapabilityGroup::Coordination,
                    "Persisted headless automation jobs with execution dispatch.",
                    Some("automation"),
                    vec!["jobs.list", "jobs.create", "jobs.run"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "automation.headless",
                    "Automation",
                    CapabilityGroup::Coordination,
                    "Headless job schedules, batch tasks, and verification execution receipts.",
                    Some("automation"),
                    "FleetAutomationRepository is not configured on this daemon instance.",
                )
            },
            if self.research.is_some() {
                CapabilityDescriptor::available(
                    "personal.notes",
                    "Notes",
                    CapabilityGroup::Personal,
                    "Task notes, scratchpads, and Markdown knowledge capture with version history.",
                    Some("notes"),
                    vec!["list", "get", "save", "history"],
                )
            } else {
                CapabilityDescriptor::unavailable(
                    "personal.notes",
                    "Notes",
                    CapabilityGroup::Personal,
                    "Task notes, scratchpads, and Markdown knowledge capture with version history.",
                    Some("notes"),
                    "ResearchRepository is not configured on this daemon instance.",
                )
            },
        ]
    }
}
