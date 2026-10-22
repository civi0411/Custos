mod assistant;
mod compute;
mod execution;
mod integrations;
mod packs;
mod providers;
mod research;
mod sessions;
mod system;
mod tasks;
mod workspaces;

use super::*;

impl LocalApiDispatcher {
    pub(super) async fn dispatch_application(&self, req: ApiRequest) -> ApiResponse {
        match req.method.split('.').nth(1).unwrap_or("") {
            // Group: Tasks & Workflow — Task lifecycle, workflow execution, omni-intelligence orchestration
            "tasks" | "workflow" | "oi" => self.handle_tasks(req).await,

            // Group: Sessions & Bridge — Interactive session management & external harness bridge
            "sessions" | "session" | "bridge" => self.handle_sessions(req).await,

            // Group: Workspaces & Git — Workspace filesystem, worktree leasing, git changes, github integration
            "workspaces" | "workspace" | "github" => self.handle_workspaces(req).await,

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

            // Group: Domain Packs — Domain-specific pack manifests, jobs and roles
            "packs" => self.handle_packs(req).await,

            // Group: System & Diagnostic — System settings, diagnostic capabilities, ping & health
            "settings" | "system" | "capabilities" | "resources" | "ping" | "health" => {
                self.handle_system(req).await
            }
            _ => ApiResponse::error(req.id.clone(), format!("Unknown method: {}", req.method)),
        }
    }
}
