use crate::api::contexts::DaemonBroadcastEvent;
use crate::api::*;
use crate::custos_local_api::*;
use chrono::Utc;
use custos_contracts::oi::{PersonaRoom, PersonaRoomMessage, PersonaRoomSenderKind};
use custos_contracts::{new_id, PersonaRegistryPort, PersonaRoomPort};

impl LocalApiDispatcher {
    pub(super) async fn handle_persona_rooms(
        &self,
        req: ApiRequest,
        store: &custos_storage::SqliteTaskStore,
    ) -> ApiResponse {
        match req.method.as_str() {
            METHOD_PERSONAS_ROOMS_CREATE => {
                let params: CreatePersonaRoomParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid room params: {e}"))
                    }
                };
                let now = Utc::now().to_rfc3339();
                let room = PersonaRoom {
                    id: new_id("proom"),
                    title: params.title,
                    member_persona_ids: params.member_persona_ids,
                    default_workbench: params.default_workbench,
                    active_session_id: None,
                    created_at: now.clone(),
                    updated_at: now,
                };
                match store.personas().create_room(&room).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "room": room })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PERSONAS_ROOMS_LIST => match store.personas().list_rooms().await {
                Ok(rooms) => ApiResponse::success(req.id, serde_json::json!({ "rooms": rooms })),
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            METHOD_PERSONAS_ROOMS_POST_MESSAGE => {
                let params: PostPersonaRoomMessageParams = match serde_json::from_value(req.params)
                {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid message params: {e}"))
                    }
                };
                let room = match store.personas().get_room(&params.room_id).await {
                    Ok(Some(r)) => r,
                    Ok(None) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Persona room '{}' not found", params.room_id),
                        )
                    }
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                let all_personas = store.personas().list_personas().await.unwrap_or_default();
                let targets = extract_room_mentions(&params.content, &room.member_persona_ids, &all_personas);

                let msg_id = new_id("pmsg");
                let root_request_id = params.root_request_id.unwrap_or_else(|| new_id("preq"));
                let msg = PersonaRoomMessage {
                    id: msg_id.clone(),
                    room_id: room.id.clone(),
                    sender: params.sender.unwrap_or_else(|| "owner".into()),
                    sender_kind: params.sender_kind.unwrap_or(PersonaRoomSenderKind::Owner),
                    target_persona_id: targets.first().cloned(),
                    root_request_id: Some(root_request_id.clone()),
                    content: params.content.clone(),
                    task_id: None,
                    run_id: None,
                    created_at: Utc::now().to_rfc3339(),
                };
                if let Err(e) = store.personas().append_room_message(&msg).await {
                    return ApiResponse::error(req.id, e.to_string());
                }

                // Dispatch governed runs for each mentioned persona
                let mut dispatched_tasks = Vec::new();
                for target_id in &targets {
                    if let Ok(task) = self
                        .kernel
                        .task_service
                        .create_task(format!("[Room: {}] @{}", room.title, target_id))
                        .await
                    {
                        dispatched_tasks.push(serde_json::json!({
                            "persona_id": target_id,
                            "task_id": task.id,
                        }));
                        if let Some(ref workflow) = self.kernel.workflow {
                            let mut cmd = custos_contracts::StartRunCommand::new(
                                task.id,
                                format!("persona:{target_id}"),
                            )
                            .with_prompt(params.content.clone());
                            cmd.persona_id = Some(target_id.clone());
                            if let Some(ref sid) = room.active_session_id {
                                cmd = cmd.with_session_id(sid.clone());
                            }
                            let _ = workflow.start_run_background(cmd).await;
                        }
                    }
                }

                let _ = self
                    .kernel
                    .event_bus
                    .send(DaemonBroadcastEvent::PersonaRoomMessageAdded {
                        persona_id: targets.first().cloned().unwrap_or_default(),
                        room_id: room.id,
                        message_id: msg_id,
                    });

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "message": msg,
                        "root_request_id": root_request_id,
                        "targeted_personas": targets,
                        "dispatched_tasks": dispatched_tasks,
                    }),
                )
            }
            METHOD_PERSONAS_ROOMS_HISTORY => {
                let params: ListPersonaRoomHistoryParams = match serde_json::from_value(req.params)
                {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid history params: {e}"))
                    }
                };
                match store
                    .personas()
                    .list_room_messages(&params.room_id, params.limit.unwrap_or(50))
                    .await
                {
                    Ok(messages) => {
                        ApiResponse::success(req.id, serde_json::json!({ "messages": messages }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown room method: {unknown}")),
        }
    }
}

/// Parses `@mention` targets while ignoring fenced code blocks and `@/path` tokens.
fn extract_room_mentions(
    content: &str,
    member_ids: &[String],
    all_personas: &[custos_contracts::AgentPersonaProfile],
) -> Vec<String> {
    let mut sanitized = String::with_capacity(content.len());
    let mut in_fence = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            sanitized.push_str(line);
            sanitized.push(' ');
        }
    }

    let lower = sanitized.to_ascii_lowercase();
    if lower.split_whitespace().any(|tok| tok == "@everyone") {
        return member_ids.to_vec();
    }

    let mut matched = Vec::new();
    for member_id in member_ids {
        let name = all_personas
            .iter()
            .find(|p| &p.id == member_id)
            .map(|p| p.name.to_ascii_lowercase());
        let id_tag = format!("@{}", member_id.to_ascii_lowercase());
        let name_tag = name.map(|n| format!("@{n}"));
        let is_hit = lower.split_whitespace().any(|tok| {
            let clean = tok.trim_end_matches([',', '.', ':', ';', '!', '?']);
            clean == id_tag || name_tag.as_deref() == Some(clean)
        });
        if is_hit {
            matched.push(member_id.clone());
        }
    }
    matched
}

