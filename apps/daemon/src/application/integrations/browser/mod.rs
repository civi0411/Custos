//! Browser session and tab automation handlers.

mod inspection;
mod interaction;
mod navigation;
mod sessions;
mod tabs;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_integrations_browser(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_BROWSER_SESSIONS_LIST | METHOD_BROWSER_SESSIONS_CREATE => {
                self.handle_browser_sessions(req).await
            }
            METHOD_BROWSER_TABS_LIST | METHOD_BROWSER_TABS_CREATE | METHOD_BROWSER_TABS_CLOSE => {
                self.handle_browser_tabs(req).await
            }
            METHOD_BROWSER_TABS_NAVIGATE | METHOD_BROWSER_TABS_SCREENSHOT => {
                self.handle_browser_navigation(req).await
            }
            METHOD_BROWSER_TABS_SNAPSHOT => self.handle_browser_snapshot(req).await,
            METHOD_BROWSER_TABS_CLICK | METHOD_BROWSER_TABS_FILL => {
                self.handle_browser_interaction(req).await
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
