//! packs application use cases over the shared daemon context.

mod package_management;
mod workbench_catalog;

use super::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_packs(&self, req: ApiRequest) -> ApiResponse {
        if req.method.starts_with("packages.")
            || req.method.starts_with("v1.packages.")
            || req.method.starts_with("custos.packages.")
        {
            self.handle_package_management(req).await
        } else {
            self.handle_workbench_catalog(req).await
        }
    }
}
