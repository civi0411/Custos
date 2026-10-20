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
            "tasks" | "workflow" | "oi" => self.handle_tasks(req).await,
            "sessions" | "session" | "bridge" => self.handle_sessions(req).await,
            "workspaces" | "workspace" | "github" => self.handle_workspaces(req).await,
            "research" | "notebook" | "reviews" | "synthesis" | "artifacts" | "notes" => {
                self.handle_research(req).await
            }
            "compute" | "experiment" => self.handle_compute(req).await,
            "assistant" => self.handle_assistant(req).await,
            "providers" | "models" | "local_models" | "oauth" | "keys" | "llm" => {
                self.handle_providers(req).await
            }
            "terminal" | "harness" => self.handle_execution(req).await,
            "browser" | "emulator" | "fleet" | "automation" => self.handle_integrations(req).await,
            "packs" => self.handle_packs(req).await,
            "settings" | "system" | "capabilities" | "resources" | "ping" | "health" => {
                self.handle_system(req).await
            }
            _ => ApiResponse::error(req.id.clone(), format!("Unknown method: {}", req.method)),
        }
    }
}
