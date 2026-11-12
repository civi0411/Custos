//! Personal Assistant Drafts Application Handlers
//!
//! Handles creating, retrieving, updating, and deleting assistant drafts.

use crate::api::*;
use custos_workbench_assistant::AssistantDraft;

pub async fn handle_list(dispatcher: &LocalApiDispatcher, req: ApiRequest) -> ApiResponse {
    let assistant = match dispatcher.assistant_repo() {
        Some(a) => a,
        None => {
            return ApiResponse::error(req.id, "AssistantRepository not configured on daemon")
        }
    };
    let session_id = req.params.get("session_id").and_then(|v| v.as_str());
    let task_id = req.params.get("task_id").and_then(|v| v.as_str());
    match assistant.list_drafts_sync(session_id, task_id) {
        Ok(drafts) => ApiResponse::success(
            req.id,
            serde_json::to_value(&drafts).unwrap_or_default(),
        ),
        Err(e) => ApiResponse::error(req.id, e.to_string()),
    }
}

pub async fn handle_get(dispatcher: &LocalApiDispatcher, req: ApiRequest) -> ApiResponse {
    let assistant = match dispatcher.assistant_repo() {
        Some(a) => a,
        None => {
            return ApiResponse::error(req.id, "AssistantRepository not configured on daemon")
        }
    };
    let id = match req.params.get("id").and_then(|v| v.as_str()) {
        Some(i) => i,
        None => return ApiResponse::error(req.id, "Missing id parameter"),
    };
    match assistant.get_draft_sync(id) {
        Ok(Some(draft)) => ApiResponse::success(
            req.id,
            serde_json::to_value(&draft).unwrap_or_default(),
        ),
        Ok(None) => ApiResponse::error(req.id, format!("Draft '{id}' not found")),
        Err(e) => ApiResponse::error(req.id, e.to_string()),
    }
}

pub async fn handle_save(dispatcher: &LocalApiDispatcher, req: ApiRequest) -> ApiResponse {
    let assistant = match dispatcher.assistant_repo() {
        Some(a) => a,
        None => {
            return ApiResponse::error(req.id, "AssistantRepository not configured on daemon")
        }
    };
    let mut params = req.params;
    let Some(fields) = params.as_object_mut() else {
        return ApiResponse::error(req.id, "Expected draft object");
    };
    let now = chrono::Utc::now().to_rfc3339();
    fields.entry("status").or_insert(serde_json::json!("draft"));
    fields.entry("created_at").or_insert(serde_json::json!(now));
    fields.insert("updated_at".into(), serde_json::json!(now));
    let mut draft: AssistantDraft = match serde_json::from_value(params) {
        Ok(d) => d,
        Err(e) => {
            return ApiResponse::error(req.id, format!("Invalid AssistantDraft: {e}"))
        }
    };
    if draft.id.is_empty()
        || draft.id.len() > 128
        || draft.title.trim().is_empty()
        || !matches!(draft.status.as_str(), "draft" | "approved" | "archived")
        || !matches!(
            draft.kind.as_str(),
            "email" | "document" | "schedule" | "note"
        )
    {
        return ApiResponse::error(req.id, "Invalid draft id, title, kind or status");
    }
    match assistant.get_draft_sync(&draft.id) {
        Ok(Some(existing)) => {
            if existing.session_id != draft.session_id
                || existing.task_id != draft.task_id
            {
                return ApiResponse::error(req.id, "Draft scope cannot be changed");
            }
            if existing.payload_lock.is_some()
                && (existing.title != draft.title
                    || existing.content != draft.content
                    || existing.recipient != draft.recipient
                    || existing.kind != draft.kind
                    || existing.status != draft.status)
            {
                return ApiResponse::error(
                    req.id,
                    "Reviewed draft is locked; create a new draft to revise it",
                );
            }
            draft.created_at = existing.created_at;
        }
        Ok(None) => {
            draft.created_at = now;
        }
        Err(error) => return ApiResponse::error(req.id, error.to_string()),
    }
    draft.payload_lock = if draft.status == "approved" {
        let payload = serde_json::json!({"title": draft.title, "content": draft.content, "kind": draft.kind, "recipient": draft.recipient});
        Some(
            blake3::hash(payload.to_string().as_bytes())
                .to_hex()
                .to_string(),
        )
    } else {
        None
    };
    match assistant.save_draft_sync(&draft) {
        Ok(()) => ApiResponse::success(
            req.id,
            serde_json::to_value(&draft).unwrap_or_default(),
        ),
        Err(e) => ApiResponse::error(req.id, e.to_string()),
    }
}

pub async fn handle_delete(dispatcher: &LocalApiDispatcher, req: ApiRequest) -> ApiResponse {
    let assistant = match dispatcher.assistant_repo() {
        Some(a) => a,
        None => {
            return ApiResponse::error(req.id, "AssistantRepository not configured on daemon")
        }
    };
    let id = match req.params.get("id").and_then(|v| v.as_str()) {
        Some(i) => i,
        None => return ApiResponse::error(req.id, "Missing id parameter"),
    };
    match assistant.delete_draft_sync(id) {
        Ok(deleted) => {
            ApiResponse::success(req.id, serde_json::json!({ "deleted": deleted }))
        }
        Err(e) => ApiResponse::error(req.id, e.to_string()),
    }
}
