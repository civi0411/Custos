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
