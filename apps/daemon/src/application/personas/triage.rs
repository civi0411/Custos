use crate::api::contexts::DaemonBroadcastEvent;
use crate::api::*;
use crate::custos_local_api::*;
use chrono::Utc;
use custos_contracts::oi::PersonaItemKind;
use custos_contracts::{PersonaTriagePort, StartRunCommand};

impl LocalApiDispatcher {
    pub(super) async fn handle_persona_triage(
        &self,
        req: ApiRequest,
        store: &custos_storage::SqliteTaskStore,
    ) -> ApiResponse {
        match req.method.as_str() {
            METHOD_PERSONAS_ITEMS_LIST => {
                let params: ListPersonaItemsParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid list params: {e}")),
                };
                match store
                    .personas()
                    .list_items(params.persona_id.as_deref(), params.bucket)
                    .await
                {
                    Ok(items) => ApiResponse::success(req.id, serde_json::json!({ "items": items })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PERSONAS_ITEMS_RESOLVE => {
                let params: ResolvePersonaItemParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid resolve params: {e}"))
                    }
                };
                let item = match store.personas().get_item(&params.item_id).await {
                    Ok(Some(i)) => i,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Persona item '{}' not found", params.item_id),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                // Sync authoritative source-of-truth table if linked
                if let Some(ref src) = item.source_ref {
                    if src.source_kind == "memory_proposal" {
                        let accept = params.decision == "accept";
                        if let Err(e) = store.memory().resolve_proposal(&src.source_id, accept) {
                            return ApiResponse::error(req.id, e.to_string());
                        }
                    }
                }

                // If starting an UpNext finding, dispatch a governed task + background run
                let mut started_task_id: Option<String> = None;
                if params.decision == "start" && item.kind == PersonaItemKind::LookAroundFinding {
                    let ask_prompt = item
                        .payload_json
                        .get("ask")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&item.summary)
                        .to_string();
                    if let Ok(task) = self
                        .kernel
                        .task_service
                        .create_task(format!("[{}] {}", item.persona_id, item.title))
                        .await
                    {
                        started_task_id = Some(task.id.clone());
                        if let Some(ref workflow) = self.kernel.workflow {
                            let mut cmd = StartRunCommand::new(
                                task.id.clone(),
                                format!("persona:{}", item.persona_id),
                            )
                            .with_prompt(ask_prompt);
                            cmd.persona_id = Some(item.persona_id.clone());
                            let _ = workflow.start_run_background(cmd).await;
                        }
                    }
                }

                let now = Utc::now().to_rfc3339();
                match store.personas().resolve_item(&params.item_id, &now).await {
                    Ok(()) => {
                        let _ = self
                            .kernel
                            .event_bus
                            .send(DaemonBroadcastEvent::PersonaItemUpdated {
                                persona_id: item.persona_id,
                                item_id: params.item_id.clone(),
                                bucket: "handled".into(),
                            });
                        ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "item_id": params.item_id,
                                "decision": params.decision,
                                "selected_options": params.selected_options,
                                "started_task_id": started_task_id,
                                "status": "handled"
                            }),
                        )
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown triage method: {unknown}")),
        }
    }
}
