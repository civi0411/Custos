//! Providers application use cases organized by domain submodules:
//! - crud: provider accounts, routing, keys, llm status
//! - models: probe, catalog, pricing, usage
//! - local_models: local inference models, folders, instances
//! - oauth: OAuth PKCE flows and callbacks

pub mod crud;
pub mod local_models;
pub mod models;
pub mod oauth;

use crate::api::*;

pub(super) fn default_model_probe_base_url(provider_type: &str) -> Option<&'static str> {
    match provider_type.to_ascii_lowercase().as_str() {
        "openai" | "codex" => Some("https://api.openai.com/v1"),
        "anthropic" | "claude" => Some("https://api.anthropic.com/v1"),
        "gemini" | "google" => Some("https://generativelanguage.googleapis.com"),
        "copilot" => Some("https://api.githubcopilot.com"),
        "antigravity" => Some("https://cloudcode-pa.googleapis.com"),
        "deepseek" => Some("https://api.deepseek.com/v1"),
        "local" | "ollama" => Some("http://localhost:11434"),
        "lmstudio" | "lm-studio" => Some("http://127.0.0.1:1234/v1"),
        _ => None,
    }
}

impl LocalApiDispatcher {
    pub(crate) async fn handle_providers(&self, req: ApiRequest) -> ApiResponse {
        let method = req.method.as_str();
        if method.starts_with("v1.local_models.") {
            self.handle_providers_local_models(req).await
        } else if method.starts_with("v1.models.") || method == "v1.providers.usage" {
            self.handle_providers_models(req).await
        } else if method.starts_with("v1.oauth.") {
            self.handle_providers_oauth(req).await
        } else {
            self.handle_providers_crud(req).await
        }
    }
}
