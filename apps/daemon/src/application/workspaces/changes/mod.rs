//! Workspace candidate spawn, compare, and integrate use cases.

pub mod compare;
pub mod integrate;
pub mod race;
pub mod spawn;

#[cfg(test)]
pub(crate) mod tests;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_workspaces_changes(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_WORKSPACES_CANDIDATES_SPAWN => self.handle_workspaces_candidates_spawn(req).await,
            METHOD_WORKSPACES_CANDIDATES_COMPARE => self.handle_workspaces_candidates_compare(req).await,
            METHOD_WORKSPACES_INTEGRATE => self.handle_workspaces_integrate(req).await,
            METHOD_WORKSPACES_CANDIDATES_RACE => self.handle_workspaces_candidates_race(req).await,
            unknown => ApiResponse::error(req.id, format!("Unknown changes method: {unknown}")),
        }
    }
}
