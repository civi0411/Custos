//! Sovereign Personas RPC handlers (`v1.personas.*`).

mod rooms;
mod triage;

use crate::api::contexts::DaemonBroadcastEvent;
use crate::api::*;
use crate::custos_local_api::*;
use crate::push::notify_persona_needs_you;
use chrono::Utc;
use custos_contracts::authority::action::RiskLevel;
use custos_contracts::oi::{
    AgentPersonaProfile, CallerIdentity, PersonaItem, PersonaItemBucket, PersonaItemKind,
};
use custos_contracts::{
    new_id, DecisionRecord, ExecutorDescriptor, ExecutorRegistryPort, MemoryPort, MemoryScope,
    PersonaRegistryPort, PersonaTriagePort, RecallQuery, StartRunCommand,
};
use custos_engine::orchestration::oi::{
    DecisionSnapshotExtractor, OiEngine, PersonaDispatcher, PersonaPlanRunner, PlanCompiler,
};
use custos_protocol::relay::PushCategory;

struct HarnessExecutorRegistry<'a> {
    harnesses: &'a custos_integrations::harness::HarnessRegistry,
}

impl ExecutorRegistryPort for HarnessExecutorRegistry<'_> {
    fn list_available_executors(&self) -> Vec<ExecutorDescriptor> {
        let mut ids: Vec<String> = self
            .harnesses
            .runtimes()
            .keys()
            .cloned()
            .collect();
        if ids.is_empty() {
            ids.push("custos-agent".to_string());
        }
        ids.into_iter()
            .map(|harness_id| ExecutorDescriptor {
                harness_id,
                supports_worktree: true,
                supports_read_only: true,
            })
            .collect()
    }
}

impl LocalApiDispatcher {
    pub(crate) async fn handle_personas(&self, req: ApiRequest) -> ApiResponse {
        let Some(store) = self.kernel.sqlite_store.as_ref() else {
            return ApiResponse::error(req.id, "SqliteTaskStore not configured");
        };
        let personas = store.personas();

        match req.method.as_str() {
            METHOD_PERSONAS_LIST => match personas.list_personas().await {
                Ok(list) => ApiResponse::success(req.id, serde_json::json!({ "personas": list })),
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            METHOD_PERSONAS_GET => {
                let Some(id) = req.params.get("persona_id").and_then(|v| v.as_str()) else {
                    return ApiResponse::error(req.id, "Missing 'persona_id'");
                };
                match personas.get_persona(id).await {
                    Ok(Some(p)) => ApiResponse::success(req.id, serde_json::json!({ "persona": p })),
                    Ok(None) => ApiResponse::error(req.id, format!("Persona '{id}' not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PERSONAS_UPSERT => {
                let profile: AgentPersonaProfile = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, format!("Invalid persona profile: {e}")),
                };
                match personas.upsert_persona(&profile).await {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "persona": profile })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_PERSONAS_ASK => self.handle_persona_ask(req, store).await,
            METHOD_PERSONAS_ITEMS_LIST | METHOD_PERSONAS_ITEMS_RESOLVE => {
                self.handle_persona_triage(req, store).await
            }
            METHOD_PERSONAS_ROOMS_CREATE
            | METHOD_PERSONAS_ROOMS_LIST
            | METHOD_PERSONAS_ROOMS_POST_MESSAGE
            | METHOD_PERSONAS_ROOMS_HISTORY => self.handle_persona_rooms(req, store).await,
            unknown => ApiResponse::error(req.id, format!("Unknown persona method: {unknown}")),
        }
    }

    async fn handle_persona_ask(
        &self,
        req: ApiRequest,
        store: &custos_storage::SqliteTaskStore,
    ) -> ApiResponse {
        let params: AskPersonaParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(e) => return ApiResponse::error(req.id, format!("Invalid ask params: {e}")),
        };
        let profile = match store.personas().get_persona(&params.persona_id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                return ApiResponse::error(req.id, format!("Persona '{}' not found", params.persona_id))
            }
            Err(e) => return ApiResponse::error(req.id, e.to_string()),
        };

        let caller = CallerIdentity {
            sender_id: params.sender_id.unwrap_or_else(|| "owner".into()),
            scope: params.caller_scope.unwrap_or_else(|| "owner_desktop".into()),
            is_paired_owner: params.is_paired_owner.unwrap_or(true),
        };
        let workbench = params
            .workbench
            .unwrap_or_else(|| profile.default_workbench.clone());
        let requested_risk = params.requested_risk.unwrap_or(RiskLevel::Low);

        let recalled = store
            .memory()
            .recall_context(
                &RecallQuery {
                    text: params.prompt.clone(),
                    limit: 5,
                    token_budget: Some(1024),
                },
                &MemoryScope {
                    persona_scope: Some(profile.memory_scope.0.clone()),
                    ..Default::default()
                },
            )
            .await
            .unwrap_or_default();

        let task = match self
            .kernel
            .task_service
            .create_task(format!("[{}] {}", profile.name, params.prompt))
            .await
        {
            Ok(t) => t,
            Err(e) => return ApiResponse::error(req.id, e.to_string()),
        };

        let exec_reg = HarnessExecutorRegistry {
            harnesses: &self.env.harnesses,
        };
        let snapshot = match DecisionSnapshotExtractor::extract_snapshot_with_executors(
            &task,
            None,
            Some(&exec_reg),
            None,
            50_000,
            5.0,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => return ApiResponse::error(req.id, e.to_string()),
        };

        let oi = OiEngine::new();
        let (proposal, authority) = match PersonaDispatcher::plan_for_persona(
            &profile,
            &caller,
            &workbench,
            requested_risk,
            &snapshot,
            &oi,
        )
        .await
        {
            Ok(res) => res,
            Err(e) => return ApiResponse::error(req.id, e.to_string()),
        };

        let (revision, placements) = PlanCompiler::compile_with_placements(&proposal, 1);
        let _ = store.decisions().save_decision(&DecisionRecord::new(
            snapshot.clone(),
            proposal.clone(),
            0,
        ));
        let _ = store
            .workflow_revisions()
            .save_revision(&revision, &placements);

        if authority.requires_biometric_approval {
            let now = Utc::now().to_rfc3339();
            let item = PersonaItem {
                id: new_id("pitem"),
                persona_id: profile.id.clone(),
                bucket: PersonaItemBucket::NeedsYou,
                kind: PersonaItemKind::PermitApproval,
                title: format!("Approval needed for {}", profile.name),
                summary: params.prompt.clone(),
                ask_questions: Vec::new(),
                source_ref: None,
                task_id: Some(task.id.clone()),
                run_id: None,
                priority: 100,
                payload_json: serde_json::json!({ "workbench": workbench.as_str() }),
                created_at: now.clone(),
                updated_at: now,
                resolved_at: None,
            };
            let _ = store.personas().upsert_item(&item).await;
            let _ = self.kernel.event_bus.send(DaemonBroadcastEvent::PersonaItemUpdated {
                persona_id: profile.id.clone(),
                item_id: item.id.clone(),
                bucket: "needs_you".into(),
            });
            if let Some(ref dev_id) = params.device_id {
                notify_persona_needs_you(
                    self.kernel.push_gateway.as_ref(),
                    dev_id,
                    PushCategory::PermitApproval,
                    1,
                )
                .await;
            }
            return ApiResponse::success(
                req.id,
                serde_json::json!({
                    "status": "awaiting_biometric_approval",
                    "item": item,
                    "task_id": task.id,
                    "topology": proposal.chosen_topology,
                    "recalled_memory_count": recalled.len(),
                }),
            );
        }

        let mut run_id: Option<String> = None;
        let mut execution_mode = "single_run";
        if let Some(ref workflow) = self.kernel.workflow {
            let mut cmd = StartRunCommand::new(task.id.clone(), format!("persona:{}", profile.id))
                .with_prompt(params.prompt.clone())
                .with_workflow_revision(revision.revision_id.clone());
            cmd.persona_id = Some(profile.id.clone());
            if let Some(ws_id) = params.workspace_id {
                cmd = cmd.with_workspace_id(ws_id);
            }
            if revision.nodes.len() > 1 {
                execution_mode = "multi_node_dag";
                let wf = workflow.clone();
                let rev = revision.clone();
                tokio::spawn(async move {
                    if let Err(err) = PersonaPlanRunner::execute_plan(wf, cmd, &rev, placements).await
                    {
                        tracing::warn!(revision_id = %rev.revision_id, error = %err, "PersonaPlanRunner DAG execution failed");
                    }
                });
            } else if let Ok(handle) = workflow.start_run_background(cmd).await {
                run_id = Some(handle.run_id);
            }
        }

        ApiResponse::success(
            req.id,
            serde_json::json!({
                "status": "dispatched",
                "execution_mode": execution_mode,
                "task_id": task.id,
                "run_id": run_id,
                "topology": proposal.chosen_topology,
                "revision_id": revision.revision_id,
                "recalled_memory_count": recalled.len(),
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_server::caller_scope::{CallerScope, CallerScopePolicy};
    use custos_contracts::oi::{ActsForTrustPolicy, MemoryScopeId, WhoCanAskPolicy};
    use custos_contracts::WorkbenchId;
    use std::sync::Arc;

    #[tokio::test]
    async fn persona_rpc_lifecycle_ask_triage_and_rooms() {
        let store = Arc::new(custos_storage::SqliteTaskStore::new_in_memory().unwrap());
        let task_service = Arc::new(TaskService::new(store.clone()));
        let session_manager = Arc::new(SessionManager::new());
        let bridge_service = Arc::new(BridgeService::new(
            session_manager.clone(),
            task_service.clone(),
        ));
        let api = LocalApiDispatcher::new(task_service, session_manager, bridge_service)
            .with_sqlite_store(store.clone());

        assert!(CallerScopePolicy::allows(CallerScope::Mobile, METHOD_PERSONAS_ASK));
        assert!(CallerScopePolicy::allows(CallerScope::Mobile, METHOD_PERSONAS_ITEMS_RESOLVE));

        let profile = AgentPersonaProfile {
            id: "coding".into(),
            name: "Coding".into(),
            role: "Engineer".into(),
            default_workbench: WorkbenchId::new(WorkbenchId::CODING),
            memory_scope: MemoryScopeId::new("persona:coding"),
            allowed_triggers: WhoCanAskPolicy::OwnerOnly,
            acts_for: ActsForTrustPolicy {
                allowed_workbenches: vec![WorkbenchId::new(WorkbenchId::CODING)],
                allowed_capabilities: vec!["fs_read".into()],
                max_autonomous_risk: RiskLevel::Low,
                require_biometric_for_mutations: true,
                allow_mobile_dispatch: true,
            },
            sovereign_instructions: "Follow Ponytail rules.".into(),
            look_around_job_id: None,
            max_up_next_items: 5,
            enabled: true,
        };
        let upsert_res = api
            .dispatch_application(ApiRequest::new(
                "r1",
                METHOD_PERSONAS_UPSERT,
                serde_json::to_value(&profile).unwrap(),
            ))
            .await;
        assert!(upsert_res.error.is_none(), "upsert failed: {:?}", upsert_res.error);

        let ask_res = api
            .dispatch_application(ApiRequest::new(
                "r2",
                METHOD_PERSONAS_ASK,
                serde_json::json!({
                    "persona_id": "coding",
                    "prompt": "Apply risky migration",
                    "caller_scope": "mobile",
                    "is_paired_owner": true,
                    "requested_risk": "high"
                }),
            ))
            .await;
        assert!(ask_res.error.is_none(), "ask failed: {:?}", ask_res.error);
        let ask_val = ask_res.result.unwrap();
        assert_eq!(ask_val["status"], "awaiting_biometric_approval");
        let item_id = ask_val["item"]["id"].as_str().unwrap().to_string();

        let resolve_res = api
            .dispatch_application(ApiRequest::new(
                "r3",
                METHOD_PERSONAS_ITEMS_RESOLVE,
                serde_json::json!({ "item_id": item_id, "decision": "accept" }),
            ))
            .await;
        assert!(resolve_res.error.is_none());

        let room_res = api
            .dispatch_application(ApiRequest::new(
                "r4",
                METHOD_PERSONAS_ROOMS_CREATE,
                serde_json::json!({ "title": "Eng Room", "member_persona_ids": ["coding"] }),
            ))
            .await;
        assert!(room_res.error.is_none());
        let room_id = room_res.result.unwrap()["room"]["id"]
            .as_str()
            .unwrap()
            .to_string();

        let post_res = api
            .dispatch_application(ApiRequest::new(
                "r5",
                METHOD_PERSONAS_ROOMS_POST_MESSAGE,
                serde_json::json!({
                    "room_id": room_id,
                    "content": "@coding review PR #180"
                }),
            ))
            .await;
        assert!(post_res.error.is_none(), "post failed: {:?}", post_res.error);
        assert_eq!(
            post_res.result.unwrap()["targeted_personas"],
            serde_json::json!(["coding"])
        );

        // Verify source_ref memory_proposal single-truth sync
        let receipt = store
            .memory()
            .propose_fact(
                custos_contracts::FactProposal {
                    subject: "persona".into(),
                    predicate: "prefers".into(),
                    object: "short_diffs".into(),
                    confidence: 0.5,
                    evidence_refs: vec![],
                    persona_scope: Some("persona:coding".into()),
                },
                "wrun-sync".into(),
            )
            .await
            .expect("propose_fact");
        let sync_item = PersonaItem {
            id: "pitem-sync-1".into(),
            persona_id: "coding".into(),
            bucket: PersonaItemBucket::NeedsYou,
            kind: PersonaItemKind::MemoryProposal,
            title: "Approve memory fact".into(),
            summary: "persona prefers short_diffs".into(),
            ask_questions: Vec::new(),
            source_ref: Some(custos_contracts::oi::PersonaItemSource {
                source_kind: "memory_proposal".into(),
                source_id: receipt.proposal_id.clone(),
            }),
            task_id: None,
            run_id: None,
            priority: 10,
            payload_json: serde_json::json!({}),
            created_at: "2026-11-28T00:00:00Z".into(),
            updated_at: "2026-11-28T00:00:00Z".into(),
            resolved_at: None,
        };
        store.personas().upsert_item(&sync_item).await.unwrap();
        let sync_res = api
            .dispatch_application(ApiRequest::new(
                "r6",
                METHOD_PERSONAS_ITEMS_RESOLVE,
                serde_json::json!({ "item_id": "pitem-sync-1", "decision": "accept" }),
            ))
            .await;
        assert!(sync_res.error.is_none(), "sync resolve failed: {:?}", sync_res.error);
        let proposals = store
            .memory()
            .list_proposals(Some(custos_contracts::ProposalStatus::Accepted), 10)
            .unwrap();
        assert!(proposals.iter().any(|p| p.proposal_id == receipt.proposal_id));
    }
}

