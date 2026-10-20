//! assistant application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_assistant(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_ASSISTANT_DRAFTS_LIST => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "AssistantRepository not configured on daemon",
                        )
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
            METHOD_ASSISTANT_DRAFTS_GET => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "AssistantRepository not configured on daemon",
                        )
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
            METHOD_ASSISTANT_DRAFTS_SAVE => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "AssistantRepository not configured on daemon",
                        )
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
            METHOD_ASSISTANT_DRAFTS_DELETE => {
                let assistant = match self.assistant.as_ref() {
                    Some(a) => a,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "AssistantRepository not configured on daemon",
                        )
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
            METHOD_ASSISTANT_OUTBOX_LIST => {
                let outbox = match self.outbox_repo() {
                    Some(o) => o,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };
                let pending = match outbox
                    .list_by_status(custos_core::contracts::storage::OutboxStatus::Pending)
                    .await
                {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let dispatching = match outbox
                    .list_by_status(custos_core::contracts::storage::OutboxStatus::Dispatching)
                    .await
                {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let receipted = match outbox
                    .list_by_status(custos_core::contracts::storage::OutboxStatus::Receipted)
                    .await
                {
                    Ok(items) => items,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let mut all = Vec::new();
                all.extend(pending);
                all.extend(dispatching);
                all.extend(receipted);
                for status in [
                    custos_core::contracts::storage::OutboxStatus::Approved,
                    custos_core::contracts::storage::OutboxStatus::Failed,
                    custos_core::contracts::storage::OutboxStatus::Uncertain,
                ] {
                    match outbox.list_by_status(status).await {
                        Ok(items) => all.extend(items),
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    }
                }
                all.sort_by(|left, right| right.created_at.cmp(&left.created_at));
                let task_filter = req.params.get("task_id").and_then(|value| value.as_str());
                let items: Vec<OutboxActionItem> = all
                    .into_iter()
                    .filter(|item| task_filter.map_or(true, |id| item.task_id == id))
                    .map(|e| {
                        let recipient_str = e
                            .recipient
                            .clone()
                            .unwrap_or_else(|| "Unknown (not stored in outbox record)".into());
                        let payload_summary_str = e
                            .payload_json
                            .as_ref()
                            .map(|p| {
                                if p.len() > 100 {
                                    format!("{}...", p.chars().take(100).collect::<String>())
                                } else {
                                    p.clone()
                                }
                            })
                            .unwrap_or_else(|| format!("Payload digest: {}", e.argument_digest));

                        OutboxActionItem {
                            id: e.id,
                            task_id: e.task_id,
                            title: format!("Outbox: {}", e.action_id),
                            action_type: e.action_id,
                            recipient: recipient_str,
                            payload_summary: payload_summary_str,
                            payload_digest: e.argument_digest,
                            status: match e.status {
                                custos_core::contracts::storage::OutboxStatus::Approved => "approved".into(),
                                custos_core::contracts::storage::OutboxStatus::Pending => {
                                    "pending_approval".into()
                                }
                                custos_core::contracts::storage::OutboxStatus::Dispatching => {
                                    "dispatching".into()
                                }
                                custos_core::contracts::storage::OutboxStatus::Receipted => {
                                    "receipted".into()
                                }
                                custos_core::contracts::storage::OutboxStatus::Failed => {
                                    "failed".into()
                                }
                                custos_core::contracts::storage::OutboxStatus::Uncertain => {
                                    "uncertain".into()
                                }
                            },
                            permit_id: Some(e.permit_id),
                            receipt_id: e.receipt.as_ref().map(|r| r.receipt_id.clone()),
                            payload_json: e.payload_json.clone(),
                            error_message: e.error_message.clone(),
                            created_at: e.created_at.to_rfc3339(),
                        }
                    })
                    .collect();
                ApiResponse::success(req.id, serde_json::to_value(&items).unwrap_or_default())
            }
            METHOD_ASSISTANT_OUTBOX_CREATE => {
                let outbox = match self.outbox_repo() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };

                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => return ApiResponse::error(req.id, "Missing task_id parameter"),
                };
                let action_type = match req
                    .params
                    .get("action_type")
                    .or_else(|| req.params.get("action_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(at) if !at.trim().is_empty() => at.to_string(),
                    _ => "dispatch_message".to_string(),
                };
                let recipient = req
                    .params
                    .get("recipient")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let payload_val = req.params.get("payload");
                let payload_json = payload_val.map(|v| v.to_string());
                let payload_str = payload_json.as_deref().unwrap_or("{}");
                let argument_digest =
                    format!("sha256:{}", custos_domain::digest(payload_str.as_bytes()));
                let permit_id = req
                    .params
                    .get("permit_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| new_id("pmt"));
                let idempotency_key = req
                    .params
                    .get("idempotency_key")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let entry_id = new_id("ob");
                let mut entry = OutboxEntry::new(
                    &entry_id,
                    &task_id,
                    &action_type,
                    &permit_id,
                    &argument_digest,
                    idempotency_key,
                    custos_core::contracts::storage::OutboxStatus::Pending,
                    chrono::Utc::now(),
                );
                entry.recipient = recipient.clone();
                entry.payload_json = payload_json.clone();

                if let Err(e) = outbox.enqueue(entry).await {
                    return ApiResponse::error(
                        req.id,
                        format!("Failed to enqueue outbox entry: {e}"),
                    );
                }

                let action_item = OutboxActionItem {
                    id: entry_id,
                    task_id,
                    title: format!("Outbox: {action_type}"),
                    action_type,
                    recipient: recipient.unwrap_or_else(|| "Unknown".into()),
                    payload_summary: payload_json
                        .clone()
                        .unwrap_or_else(|| format!("Payload digest: {argument_digest}")),
                    payload_digest: argument_digest,
                    status: "pending_approval".into(),
                    permit_id: Some(permit_id),
                    receipt_id: None,
                    payload_json,
                    error_message: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                };

                ApiResponse::success(
                    req.id,
                    serde_json::to_value(&action_item).unwrap_or_default(),
                )
            }
            METHOD_ASSISTANT_OUTBOX_APPROVE => {
                let outbox = match self.outbox_repo() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };

                let entry_id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("outbox_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing outbox id parameter"),
                };

                match outbox.get(entry_id).await {
                    Ok(Some(entry)) => {
                        if let Err(error) = outbox.mark_approved(entry_id).await {
                            return ApiResponse::error(req.id, format!("Outbox approval failed: {error}"));
                        }
                        ApiResponse::success(req.id, serde_json::json!({
                            "id": entry.id,
                            "task_id": entry.task_id,
                            "status": "approved",
                            "action_id": entry.action_id,
                        }))
                    },
                    Ok(None) => {
                        ApiResponse::error(req.id, format!("Outbox entry '{entry_id}' not found"))
                    }
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to read outbox entry: {e}"))
                    }
                }
            }
            METHOD_ASSISTANT_OUTBOX_DISPATCH => {
                let outbox = match self.outbox_repo() {
                    Some(repo) => repo,
                    None => return ApiResponse::error(req.id, "Outbox storage is not configured"),
                };

                let entry_id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("outbox_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing outbox id parameter"),
                };

                let entry = match outbox.get(entry_id).await {
                    Ok(Some(e)) => e,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Outbox entry '{entry_id}' not found"),
                        )
                    }
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Failed to read outbox entry: {e}"),
                        )
                    }
                };

                if entry.status != custos_core::contracts::storage::OutboxStatus::Approved {
                    return ApiResponse::error(
                        req.id,
                        format!("Outbox entry '{entry_id}' requires durable approval before dispatch; current status is '{:?}'", entry.status),
                    );
                }

                let payload_str = entry.payload_json.as_deref().unwrap_or("{}");
                let actual_digest = format!("sha256:{}", custos_domain::digest(payload_str.as_bytes()));
                if entry.argument_digest != actual_digest {
                    return ApiResponse::error(
                        req.id,
                        format!("Outbox entry '{entry_id}' argument digest mismatch: recorded '{}', calculated '{}'. Dispatch blocked.", entry.argument_digest, actual_digest),
                    );
                }

                // T3: CAS transition to Dispatching (fences duplicate dispatch)
                if let Err(e) = outbox.mark_dispatching(entry_id).await {
                    return ApiResponse::error(
                        req.id,
                        format!("CAS transition failed for outbox entry '{entry_id}': {e}"),
                    );
                }

                // T4: Execute outbound action with durable receipt verification
                let started = std::time::Instant::now();
                let recipient = entry.recipient.as_deref().unwrap_or("unknown");

                // Execute based on recipient type (fail-closed on unsupported schemes)
                let exec_result: Result<serde_json::Value, String> = if recipient
                    .starts_with("mcp://")
                {
                    let tool_path = recipient.trim_start_matches("mcp://");
                    let namespaced_tool = tool_path.replace('/', "__");
                    let args: Option<serde_json::Value> = serde_json::from_str(payload_str).ok();
                    let ctx = custos_adapters::mcp::ToolCallContext::new(&entry.id, &entry.id);

                    match self
                        .mcp_hub
                        .dispatch_call(&namespaced_tool, args, &ctx)
                        .await
                    {
                        Ok(res) => {
                            if res.is_error == Some(true) {
                                Err(format!(
                                    "MCP tool '{}' returned error: {:?}",
                                    namespaced_tool, res.content
                                ))
                            } else {
                                Ok(serde_json::to_value(&res).unwrap_or_else(|_| {
                                    serde_json::json!({
                                        "success": true,
                                        "tool": namespaced_tool
                                    })
                                }))
                            }
                        }
                        Err(DomainError::Validation(msg)) if msg.contains("timed out") => {
                            // Timeout: Mark Uncertain per Gate 3
                            let _ = outbox.mark_uncertain(entry_id).await;
                            return ApiResponse::error(
                                req.id,
                                format!("MCP tool call timed out; marked uncertain: {msg}"),
                            );
                        }
                        Err(e) => Err(format!("MCP dispatch failed: {e}")),
                    }
                } else if recipient.starts_with("http://") || recipient.starts_with("https://") {
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(15))
                        .build()
                        .unwrap_or_default();
                    let body: serde_json::Value = serde_json::from_str(payload_str)
                        .unwrap_or_else(|_| serde_json::json!({"payload": payload_str}));
                    match client.post(recipient).json(&body).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            let json_val = resp
                                .json::<serde_json::Value>()
                                .await
                                .unwrap_or_else(|_| serde_json::json!({"status": "ok"}));
                            Ok(json_val)
                        }
                        Ok(resp) => Err(format!(
                            "HTTP dispatch failed with status: {}",
                            resp.status()
                        )),
                        Err(e) => {
                            // Timeout or connection drop: Invariant Gate 3 Uncertain
                            let _ = outbox.mark_uncertain(entry_id).await;
                            return ApiResponse::error(req.id, format!("Dispatch connection dropped or timed out; marked uncertain: {e}"));
                        }
                    }
                } else if recipient.starts_with("a2a://") {
                    let target_url = recipient.trim_start_matches("a2a://");
                    let endpoint = if target_url.starts_with("http://") || target_url.starts_with("https://") {
                        target_url.to_string()
                    } else {
                        format!("http://{target_url}")
                    };

                    let a2a_client = custos_adapters::a2a::A2aClient::new();
                    let task_payload: custos_adapters::a2a::A2aTaskPayload = serde_json::from_str(payload_str)
                        .unwrap_or_else(|_| custos_adapters::a2a::A2aTaskPayload {
                            task_id: entry.action_id.clone(),
                            context_id: None,
                            state: custos_adapters::a2a::A2aTaskState::Submitted,
                            artifacts: vec![serde_json::json!({ "raw": payload_str })],
                            parent_task_id: None,
                            parent_run_id: None,
                            generation: None,
                            budget: None,
                            error_message: None,
                        });

                    match a2a_client.delegate_task(&endpoint, &task_payload, None).await {
                        Ok(res) => {
                            if res.state == custos_adapters::a2a::A2aTaskState::Failed {
                                Err(res.error_message.unwrap_or_else(|| "A2A sub-task reported failure".to_string()))
                            } else {
                                Ok(serde_json::to_value(&res).unwrap_or_else(|_| serde_json::json!({
                                    "success": true,
                                    "task_id": res.task_id,
                                    "state": res.state.as_str()
                                })))
                            }
                        }
                        Err(DomainError::Validation(msg)) if msg.contains("timed out") => {
                            let _ = outbox.mark_uncertain(entry_id).await;
                            return ApiResponse::error(req.id, format!("A2A delegation timed out; marked uncertain: {msg}"));
                        }
                        Err(e) => Err(format!("A2A dispatch failed: {e}")),
                    }
                } else {
                    let err = format!("Unsupported outbox recipient protocol '{recipient}'. Fail-closed: mediated dispatch only supports mcp://, a2a://, and http(s):// destinations");
                    let _ = outbox.mark_failed(entry_id, &err).await;
                    return ApiResponse::error(req.id, err);
                };

                match exec_result {
                    Ok(res_val) => {
                        let duration_ms = started.elapsed().as_millis() as u64;
                        let out_str = res_val.to_string();
                        let receipt = ExecutionReceipt {
                            receipt_id: new_id("rcpt"),
                            permit_id: entry.permit_id.clone(),
                            action_id: entry.action_id.clone(),
                            status: ReceiptStatus::Success,
                            output_digest: format!(
                                "sha256:{}",
                                custos_domain::digest(out_str.as_bytes())
                            ),
                            output_data: Some(res_val),
                            error_message: None,
                            duration_ms: Some(duration_ms),
                            executed_at: chrono::Utc::now(),
                            assurance: Assurance::CustosMediated,
                        };

                        if let Err(e) = outbox.mark_receipted(entry_id, receipt.clone()).await {
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to record receipt in outbox: {e}"),
                            );
                        }

                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "id": entry_id,
                                "status": "receipted",
                                "receipt": receipt,
                            }),
                        )
                    }
                    Err(err_msg) => {
                        let _ = outbox.mark_failed(entry_id, &err_msg).await;
                        ApiResponse::error(
                            req.id,
                            format!("Outbox dispatch execution failed: {err_msg}"),
                        )
                    }
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
