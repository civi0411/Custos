mod assistant;
mod compute;
mod execution;
mod integrations;
mod packs;
mod providers;
mod quant;
mod research;
mod sessions;
mod system;
mod tasks;
mod workspaces;

pub(crate) use crate::api::*;
pub(crate) use custos_domain::DomainError;

impl LocalApiDispatcher {
    pub(crate) async fn dispatch_application(&self, req: ApiRequest) -> ApiResponse {
        let parts: Vec<&str> = req.method.split('.').collect();
        let domain_prefix = match parts.as_slice() {
            ["v1" | "custos", domain, ..] => *domain,
            [domain, ..] => *domain,
            [] => "",
        };
        match domain_prefix {
            // Group: Tasks & Workflow — Task lifecycle, workflow execution, omni-intelligence orchestration
            "tasks" | "workflow" | "oi" => self.handle_tasks(req).await,

            // Group: Sessions & Bridge — Interactive session management & external harness bridge
            "sessions" | "session" | "bridge" => self.handle_sessions(req).await,

            // Group: Workspaces & Git — Workspace filesystem, worktree leasing, git changes, github integration
            "workspaces" | "workspace" | "github" => self.handle_workspaces(req).await,

            // Group: Quantitative Trading — Backtests, tearsheets, risk audits & execution order tickets
            "quant" => self.handle_quant(req).await,

            // Group: Research — Scientific research pipeline, claims, notebook kernels, reviews & synthesis
            "research" | "notebook" | "reviews" | "synthesis" | "artifacts" | "notes" => {
                self.handle_research(req).await
            }

            // Group: Compute & Experiments — Remote compute targets, GPU leasing, experiment execution
            "compute" | "experiment" => self.handle_compute(req).await,

            // Group: Assistant & Outbox — Personal assistant drafts, outbox approvals and messaging
            "assistant" => self.handle_assistant(req).await,

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

            // Group: System & Diagnostic — System settings, diagnostic capabilities, ping & health, pipeline status
            "settings" | "system" | "capabilities" | "resources" | "ping" | "health" | "pipeline" => {
                self.handle_system(req).await
            }
            _ => ApiResponse::error(req.id.clone(), format!("Unknown method: {}", req.method)),
        }
    }
}
