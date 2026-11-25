//! Research recipes, synthesis proposals, and coding handoff use cases.

pub mod handoff;
pub mod proposals;
pub mod recipes;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_research_reviews(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            "v1.research.recipes.list" | "v1.research.recipes.save" | "v1.research.recipes.get" => {
                self.handle_research_recipes(req).await
            }
            METHOD_SYNTHESIS_PROPOSALS_LIST
            | METHOD_SYNTHESIS_PROPOSALS_GET
            | METHOD_SYNTHESIS_PROPOSALS_SAVE => self.handle_research_proposals(req).await,
            METHOD_RESEARCH_HANDOFF_CODING | METHOD_SYNTHESIS_HANDOFF_EXECUTE => {
                self.handle_research_handoff(req).await
            }
            unknown => ApiResponse::error(
                req.id,
                format!("Unknown research reviews method: {unknown}"),
            ),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
