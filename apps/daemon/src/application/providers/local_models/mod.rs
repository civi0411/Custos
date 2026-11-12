//! Local inference models, folder scan, hardware profiling, and instance management.

mod folders;
mod lifecycle;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_providers_local_models(&self, req: ApiRequest) -> ApiResponse {
        if req.method.starts_with("v1.local_models.folders.")
            || req.method == "v1.local_models.scan"
            || req.method == "v1.local_models.list"
            || req.method == "v1.local_models.hardware"
        {
            self.handle_local_models_folders(req)
        } else {
            self.handle_local_models_lifecycle(req).await
        }
    }
}
