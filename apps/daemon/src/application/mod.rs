#[cfg(feature = "custos-workbench-assistant")]
mod assistant;
pub mod blocks;
pub mod capabilities;
mod compute;
mod environment;
mod execution;
mod integrations;
mod oi;
mod packs;
mod pipeline;
mod providers;
#[cfg(feature = "custos-workbench-quant")]
mod quant;
#[cfg(feature = "custos-workbench-research")]
mod research;
mod sessions;
mod shared_capabilities;
mod system;
mod tasks;
mod workflow;
mod workspaces;

pub(crate) use crate::api::{ApiRequest, ApiResponse, LocalApiDispatcher};
pub(crate) use crate::custos_local_api::*;
pub(crate) use custos_core::contracts::TerminalPort;
pub(crate) use custos_core::*;
pub(crate) use custos_domain::*;
pub use blocks::*;

impl LocalApiDispatcher {
    pub(crate) async fn dispatch_application(&self, req: ApiRequest) -> ApiResponse {
        // Legacy UI routes annotations through v1.research.annotations.*, which is Ring 2.
        if matches!(
            req.method.as_str(),
            "v1.research.annotations.list" | "v1.research.annotations.save"
        ) {
            return self.handle_shared_artifacts(req).await;
        }
        let parts: Vec<&str> = req.method.split('.').collect();
        let domain_prefix = match parts.as_slice() {
            ["v1" | "custos", domain, ..] => *domain,
            [domain, ..] => *domain,
            [] => "",
        };
        match domain_prefix {
            // Group: Tasks — Task lifecycle & spans
            "tasks" => self.handle_tasks(req).await,

            // Group: Workflow — Workflow execution & runs
            "workflow" => self.handle_workflow(req).await,

            // Group: Omni-Intelligence — Orchestration explanations & planning
            "oi" => self.handle_oi(req).await,

            // Group: Sessions & Bridge — Interactive session management & external harness bridge
            "sessions" | "session" | "bridge" => self.handle_sessions(req).await,

            // Group: Workspaces & Git — Workspace filesystem, worktree leasing, git changes, github integration
            "workspaces" | "workspace" | "github" => self.handle_workspaces(req).await,

            // Group: Environment — Sovereign Agent Execution Environment (AEE)
            "environment" => self.handle_environment(req).await,

            // Group: Quantitative Trading — Backtests, tearsheets, risk audits & execution order tickets
            #[cfg(feature = "custos-workbench-quant")]
            "quant" => self.handle_quant(req).await,
            #[cfg(not(feature = "custos-workbench-quant"))]
            "quant" => ApiResponse::error(
                req.id,
                "pack_disabled: Domain 'quant' is not installed or disabled under lite profile",
            ),

            // Group: Ring 2 Shared Capabilities — Cross-workbench notebook kernels, reviews, artifacts & notes
            "notebook" | "reviews" | "artifacts" | "notes" => {
                self.handle_shared_capabilities(req).await
            }

            // Group: Research — Scientific research pipeline, claims, recipes & synthesis
            #[cfg(feature = "custos-workbench-research")]
            "research" | "synthesis" => self.handle_research(req).await,
            #[cfg(not(feature = "custos-workbench-research"))]
            "research" | "synthesis" => ApiResponse::error(
                req.id,
                "pack_disabled: Domain 'research' is not installed or disabled under lite profile",
            ),

            // Group: Compute & Experiments — Remote compute targets, GPU leasing, experiment execution
            "compute" | "experiment" => self.handle_compute(req).await,

            // Group: Assistant & Outbox — Personal assistant drafts, outbox approvals and messaging
            #[cfg(feature = "custos-workbench-assistant")]
            "assistant" => self.handle_assistant(req).await,
            #[cfg(not(feature = "custos-workbench-assistant"))]
            "assistant" => ApiResponse::error(
                req.id,
                "pack_disabled: Domain 'assistant' is not installed or disabled under lite profile",
            ),

            // Group: Providers & Models — LLM provider accounts, credentials, catalog, local inference
            "providers" | "models" | "local_models" | "oauth" | "keys" | "llm" => {
                self.handle_providers(req).await
            }

            // Group: Execution & PTY — PTY terminals and agent harness runners
            "terminal" | "harness" => self.handle_execution(req).await,

            // Group: Integrations — Browser/emulator automation, fleet SSH orchestration
            "browser" | "emulator" | "fleet" | "automation" => self.handle_integrations(req).await,

            // Group: Domain Packs — Domain-specific pack manifests, jobs, packages inventory
            "packs" | "packages" => self.handle_packs(req).await,

            // Group: Pipeline — Universal sovereign data pipeline (Bronze/Silver/Gold)
            "pipeline" => self.handle_pipeline(req).await,

            // Group: System & Diagnostic — System settings, diagnostic capabilities, ping & health
            "settings" | "system" | "capabilities" | "resources" | "ping" | "health" => {
                self.handle_system(req).await
            }

            // Group: Supervision & Watchdog — Aliased to workflow telemetry
            "supervision" => self.handle_workflow(req).await,
            _ => ApiResponse::error(req.id.clone(), format!("Unknown method: {}", req.method)),
        }
    }
}
