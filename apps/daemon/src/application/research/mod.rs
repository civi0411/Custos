//! Research-workbench application handlers (sources, claims, experiment runs, recipes & synthesis).

pub mod reviews;
pub mod runs;
pub mod sources;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_research(&self, req: ApiRequest) -> ApiResponse {
        if let Err(denied) = self.require_pack_enabled("research", &req.id) {
            return denied;
        }
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
            | "v1.research.executions.list"
            | "v1.research.executions.get"
            | "v1.research.executions.resume.assess" => self.handle_research_runs(req).await,

            "v1.research.recipes.list"
            | "v1.research.recipes.save"
            | "v1.research.recipes.get"
            | METHOD_RESEARCH_HANDOFF_CODING
            | METHOD_SYNTHESIS_PROPOSALS_LIST
            | METHOD_SYNTHESIS_PROPOSALS_GET
            | METHOD_SYNTHESIS_PROPOSALS_SAVE
            | METHOD_SYNTHESIS_HANDOFF_EXECUTE => self.handle_research_reviews(req).await,

            unknown => ApiResponse::error(
                req.id,
                format!("Unknown research handler method: {unknown}"),
            ),
        }
    }
}
