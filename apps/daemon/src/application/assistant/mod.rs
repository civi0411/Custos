//! Personal Assistant Application Module
//!
//! Subdivides into drafts and outbox approval handlers.

pub mod drafts;
pub mod outbox;

use super::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_assistant(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_ASSISTANT_DRAFTS_LIST => drafts::handle_list(self, req).await,
            METHOD_ASSISTANT_DRAFTS_GET => drafts::handle_get(self, req).await,
            METHOD_ASSISTANT_DRAFTS_SAVE => drafts::handle_save(self, req).await,
            METHOD_ASSISTANT_DRAFTS_DELETE => drafts::handle_delete(self, req).await,
            METHOD_ASSISTANT_OUTBOX_LIST => outbox::handle_list(self, req).await,
            METHOD_ASSISTANT_OUTBOX_CREATE => outbox::handle_create(self, req).await,
            METHOD_ASSISTANT_OUTBOX_APPROVE => outbox::handle_approve(self, req).await,
            METHOD_ASSISTANT_OUTBOX_DISPATCH => outbox::handle_dispatch(self, req).await,
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
