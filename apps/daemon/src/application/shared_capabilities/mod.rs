//! Ring 2 Shared Capabilities Use Cases (Notebooks, Reviews, Artifacts, Annotations & Notes).

pub mod artifacts;
pub mod notebooks;
pub mod reviewer;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_shared_capabilities(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_NOTEBOOK_CELLS_LIST
            | METHOD_NOTEBOOK_CELLS_SAVE
            | METHOD_NOTEBOOK_EXECUTE
            | METHOD_NOTEBOOK_INTERRUPT
            | METHOD_NOTEBOOK_RESET
            | METHOD_NOTEBOOK_STATUS => self.handle_shared_notebooks(req).await,

            METHOD_REVIEWS_LIST
            | METHOD_REVIEWS_GET
            | METHOD_REVIEWS_RECORD
            | METHOD_REVIEWS_MARK_STALE => self.handle_shared_reviews(req).await,

            METHOD_ARTIFACTS_LIST
            | METHOD_ARTIFACTS_GET
            | METHOD_ARTIFACTS_RECORD_LINEAGE
            | METHOD_ARTIFACTS_LINEAGE_GRAPH
            | METHOD_NOTES_LIST
            | METHOD_NOTES_GET
            | METHOD_NOTES_SAVE
            | METHOD_NOTES_HISTORY
            | "v1.artifacts.annotations.list"
            | "v1.artifacts.annotations.save"
            | "v1.research.annotations.list"
            | "v1.research.annotations.save" => self.handle_shared_artifacts(req).await,

            unknown => ApiResponse::error(
                req.id,
                format!("Unknown shared capability method: {unknown}"),
            ),
        }
    }
}

