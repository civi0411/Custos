//! Research application use cases organized by domain submodules:
//! - sources: sources, passage anchors, claims, and atomic ingestion
//! - notebooks: experiment runs, notebook cells, and execution lifecycle
//! - reviews: claim verifications, recipes, proposals, and coding handoffs
//! - artifacts: artifacts, lineage, annotations, and notes

pub mod artifacts;
pub mod notebooks;
pub mod reviews;
pub mod sources;

use crate::api::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_research(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_RESEARCH_SOURCES_LIST
            | METHOD_RESEARCH_SOURCES_SAVE
            | METHOD_RESEARCH_ANCHORS_LIST
            | METHOD_RESEARCH_ANCHORS_SAVE
            | METHOD_RESEARCH_CLAIMS_LIST
            | METHOD_RESEARCH_CLAIMS_SAVE
            | METHOD_RESEARCH_LINEAGE_LIST => self.handle_research_sources(req).await,

            METHOD_RESEARCH_RUNS_LIST
            | METHOD_RESEARCH_RUNS_SAVE
            | METHOD_NOTEBOOK_CELLS_LIST
            | METHOD_NOTEBOOK_CELLS_SAVE
            | METHOD_NOTEBOOK_EXECUTE
            | METHOD_NOTEBOOK_INTERRUPT
            | METHOD_NOTEBOOK_RESET
            | METHOD_NOTEBOOK_STATUS
            | "v1.research.executions.list"
            | "v1.research.executions.get"
            | "v1.research.executions.resume.assess" => self.handle_research_notebooks(req).await,

            METHOD_RESEARCH_HANDOFF_CODING
            | "v1.research.recipes.list"
            | "v1.research.recipes.save"
            | "v1.research.recipes.get"
            | METHOD_REVIEWS_LIST
            | METHOD_REVIEWS_GET
            | METHOD_REVIEWS_RECORD
            | METHOD_REVIEWS_MARK_STALE
            | METHOD_SYNTHESIS_PROPOSALS_LIST
            | METHOD_SYNTHESIS_PROPOSALS_GET
            | METHOD_SYNTHESIS_PROPOSALS_SAVE
            | METHOD_SYNTHESIS_HANDOFF_EXECUTE => self.handle_research_reviews(req).await,

            "v1.research.annotations.list"
            | "v1.research.annotations.save"
            | METHOD_ARTIFACTS_LIST
            | METHOD_ARTIFACTS_GET
            | METHOD_ARTIFACTS_RECORD_LINEAGE
            | METHOD_ARTIFACTS_LINEAGE_GRAPH
            | METHOD_NOTES_LIST
            | METHOD_NOTES_GET
            | METHOD_NOTES_SAVE
            | METHOD_NOTES_HISTORY => self.handle_research_artifacts(req).await,

            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
