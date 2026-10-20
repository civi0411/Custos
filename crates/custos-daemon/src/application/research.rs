//! research application use cases over the shared daemon context.

use super::super::*;

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_research(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_RESEARCH_SOURCES_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_sources() {
                    Ok(sources) => match serde_json::to_value(&sources) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_SOURCES_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let src: SourceRecord = match serde_json::from_value(req.params) {
                    Ok(s) => s,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid source payload: {e}"))
                    }
                };
                let file_bytes = src.local_path.as_ref().and_then(|p| std::fs::read(p).ok());
                let src = match custos_core::research_ingress::source_draft_with_bytes(
                    src,
                    file_bytes.as_deref(),
                ) {
                    Ok(src) => src,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_source(&src) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": src.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let source_id = match req.params.get("source_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing source_id parameter"),
                };
                match research.list_anchors_for_source(source_id) {
                    Ok(anchors) => match serde_json::to_value(&anchors) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_ANCHORS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let anchor: PassageAnchor = match serde_json::from_value(req.params) {
                    Ok(a) => a,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid anchor payload: {e}"))
                    }
                };
                let anchor = match custos_core::research_ingress::passage_draft(anchor) {
                    Ok(anchor) => anchor,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_sources() {
                    Ok(sources) if sources.iter().any(|s| s.id == anchor.source_id) => {}
                    Ok(_) => return ApiResponse::error(req.id, "Passage source does not exist"),
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_anchor(&anchor) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": anchor.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_claims() {
                    Ok(claims) => match serde_json::to_value(&claims) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_CLAIMS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim: ResearchClaim = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid claim payload: {e}"))
                    }
                };
                let claim = match custos_core::research_ingress::claim_draft(claim) {
                    Ok(claim) => claim,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.save_claim(&claim) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": claim.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_RUNS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_runs() {
                    Ok(runs) => match serde_json::to_value(&runs) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_RUNS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let run: ResearchExperimentRun = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid run payload: {e}"))
                    }
                };
                let run = match custos_core::research_ingress::experiment_draft(run) {
                    Ok(run) => run,
                    Err(e) => return ApiResponse::error(req.id, e),
                };
                match research.list_runs() {
                    Ok(runs) if runs.iter().any(|existing| existing.run_id == run.run_id) => {
                        return ApiResponse::error(req.id, "Experiment draft id already exists");
                    }
                    Ok(_) => {}
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                }
                match research.save_run(&run) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "run_id": run.run_id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_LINEAGE_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let artifact_path = match req.params.get("artifact_path").and_then(|v| v.as_str()) {
                    Some(path) => path,
                    None => return ApiResponse::error(req.id, "Missing artifact_path parameter"),
                };
                match research.list_artifact_lineage(artifact_path) {
                    Ok(lineage) => match serde_json::to_value(&lineage) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_RESEARCH_HANDOFF_CODING => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let claim_id = match req.params.get("claim_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing claim_id parameter"),
                };
                let claim = match research.list_claims() {
                    Ok(claims) => match claims.into_iter().find(|c| c.id == claim_id) {
                        Some(claim) => claim,
                        None => return ApiResponse::error(req.id, "Claim does not exist"),
                    },
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let title = format!("Investigate research claim: {}", claim.statement);

                let cmd = CreateTask {
                    title,
                    metadata: Some(serde_json::json!({
                        "source": "research_workbench",
                        "claim_id": claim_id,
                        "claim_statement": claim.statement,
                        "claim_grounding": "unverified_client_draft",
                    })),
                    contract: None,
                };

                match self.task_service.execute_create(cmd).await {
                    Ok((task, _event)) => match serde_json::to_value(&task) {
                        Ok(val) => ApiResponse::success(
                            req.id,
                            serde_json::json!({
                                "handoff_status": "task_created",
                                "task": val,
                            }),
                        ),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                match research.list_recipes() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.save" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe: custos_domain::Recipe = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid recipe payload: {e}"))
                    }
                };
                match research.save_recipe(&recipe) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": recipe.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.get" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.get_recipe(recipe_id) {
                    Ok(Some(recipe)) => match serde_json::to_value(&recipe) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Recipe {recipe_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.executions.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.list_execution_records_for_recipe(recipe_id) {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(value) => ApiResponse::success(req.id, value),
                        Err(error) => ApiResponse::error(req.id, error.to_string()),
                    },
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.research.executions.get" | "v1.research.executions.resume.assess" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let execution_id = match req.params.get("execution_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id,
                    _ => return ApiResponse::error(req.id, "Missing execution_id parameter"),
                };
                match research.get_execution_record(execution_id) {
                    Ok(Some(record)) => {
                        match if req.method == "v1.research.executions.resume.assess" {
                            serde_json::to_value(record.assess_checkpoint_resume())
                        } else {
                            serde_json::to_value(&record)
                        } {
                            Ok(value) => ApiResponse::success(req.id, value),
                            Err(error) => ApiResponse::error(req.id, error.to_string()),
                        }
                    }
                    Ok(None) => ApiResponse::error(
                        req.id,
                        format!("Execution record {execution_id} not found"),
                    ),
                    Err(error) => ApiResponse::error(req.id, error.to_string()),
                }
            }
            "v1.research.annotations.list" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let artifact_id = req
                    .params
                    .get("artifact_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let version = req
                    .params
                    .get("version")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                match research.list_annotations_for_artifact(artifact_id, version) {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.annotations.save" => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let anno: custos_domain::AnnotationRecord = match serde_json::from_value(req.params)
                {
                    Ok(a) => a,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid annotation payload: {e}"),
                        )
                    }
                };
                if anno.id.trim().is_empty()
                    || anno.note.trim().is_empty()
                    || anno.note.len() > 16_384
                    || anno.actor.trim().is_empty()
                    || anno.side_chat_session_id.is_some()
                    || anno.submitted_turn_id.is_some()
                {
                    return ApiResponse::error(req.id, "Annotation requires an identity, actor and note; conversation links must come from a real handoff");
                }
                let versions = match research.list_artifact_lineage(&anno.artifact_path) {
                    Ok(versions) => versions,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                if !versions
                    .iter()
                    .any(|version| version.version == anno.artifact_version)
                {
                    return ApiResponse::error(
                        req.id,
                        "Annotation must target a recorded artifact version",
                    );
                }
                let valid_target = match &anno.target {
                    custos_domain::AnnotationTarget::Text {
                        start_offset,
                        end_offset,
                        exact_text,
                    } => {
                        start_offset < end_offset
                            && !exact_text.is_empty()
                            && exact_text.len() <= 65_536
                    }
                    custos_domain::AnnotationTarget::ImagePin {
                        x_ratio,
                        y_ratio,
                        pin_number,
                    } => {
                        x_ratio.is_finite()
                            && y_ratio.is_finite()
                            && (0.0..=1.0).contains(x_ratio)
                            && (0.0..=1.0).contains(y_ratio)
                            && *pin_number > 0
                    }
                    custos_domain::AnnotationTarget::DomElement { selector } => {
                        !selector.trim().is_empty() && selector.len() <= 4096
                    }
                };
                if !valid_target || anno.status != custos_domain::AnnotationStatus::Pending {
                    return ApiResponse::error(req.id, "Annotations enter as pending notes with a valid target; they do not certify review completion");
                }
                match research.save_annotation(&anno) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": anno.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_all_artifacts() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let artifact_path = match req.params.get("artifact_path").and_then(|v| v.as_str()) {
                    Some(p) if !p.trim().is_empty() => p,
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Missing or empty artifact_path parameter",
                        )
                    }
                };
                let versions = match research.list_artifact_lineage(artifact_path) {
                    Ok(v) => v,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let annotations = match research.list_annotations_for_artifact(artifact_path, None)
                {
                    Ok(a) => a,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let (latest_version, latest_content_hash) = if let Some(latest) = versions.last() {
                    (latest.version, latest.content_hash.clone())
                } else {
                    (0, String::new())
                };

                let mut content: Option<String> = None;
                let mut content_version = None;
                let mut file = None;
                if artifact_path.starts_with("notes/") {
                    let note_id = artifact_path
                        .trim_start_matches("notes/")
                        .trim_end_matches(".md");
                    match research.get_note(note_id) {
                        Ok(Some(note)) => {
                            content_version = Some(note.version);
                            content = Some(note.content);
                        }
                        Ok(None) => {}
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    }
                } else if let Some(workspace_id) = req
                    .params
                    .get("workspace_id")
                    .and_then(|value| value.as_str())
                {
                    if versions.is_empty() {
                        return ApiResponse::error(req.id, "Artifact has no recorded lineage");
                    }
                    let workspace = match self
                        .resolve_workspace_for_dispatch(&req.id, workspace_id)
                        .await
                    {
                        Ok(workspace) => workspace,
                        Err(response) => return response,
                    };
                    let artifact = std::path::Path::new(artifact_path);
                    let relative = if artifact.is_absolute() {
                        match artifact.strip_prefix(&workspace.path) {
                            Ok(relative) => relative.to_string_lossy().to_string(),
                            Err(_) => {
                                return ApiResponse::error(
                                    req.id,
                                    "Artifact is outside the selected workspace",
                                )
                            }
                        }
                    } else {
                        artifact_path.to_string()
                    };
                    let current = match self
                        .files
                        .read_file(&workspace, &relative, Some(10 * 1024 * 1024))
                        .await
                    {
                        Ok(current) => current,
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                    };
                    let bytes = if current.is_binary {
                        match base64::engine::general_purpose::STANDARD.decode(&current.content) {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                return ApiResponse::error(
                                    req.id,
                                    format!("Invalid binary artifact encoding: {error}"),
                                )
                            }
                        }
                    } else {
                        current.content.as_bytes().to_vec()
                    };
                    let hash = blake3::hash(&bytes).to_hex().to_string();
                    if !current.truncated
                        && (latest_content_hash == hash
                            || latest_content_hash == format!("blake3:{hash}"))
                    {
                        content_version = Some(latest_version);
                        if !current.is_binary {
                            content = Some(current.content.clone());
                        }
                    }
                    file = Some(current);
                }

                let resp = ArtifactDetailResponse {
                    artifact_path: artifact_path.to_string(),
                    latest_version,
                    latest_content_hash,
                    versions,
                    annotations,
                    content,
                    file,
                    content_version,
                };
                match serde_json::to_value(&resp) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_RECORD_LINEAGE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let node: ArtifactLineageNode = match serde_json::from_value(req.params) {
                    Ok(n) => n,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid artifact lineage payload: {e}"),
                        )
                    }
                };
                if node.artifact_path.trim().is_empty() || node.content_hash.trim().is_empty() {
                    return ApiResponse::error(
                        req.id,
                        "Artifact path and content hash cannot be empty",
                    );
                }
                match research.record_artifact_lineage(&node) {
                    Ok(()) => ApiResponse::success(req.id, serde_json::json!({ "recorded": true })),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_ARTIFACTS_LINEAGE_GRAPH => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let filter_path = req.params.get("artifact_path").and_then(|v| v.as_str());
                match research.get_artifact_lineage_graph(filter_path) {
                    Ok(graph) => match serde_json::to_value(&graph) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let session_id = req.params.get("session_id").and_then(|v| v.as_str());
                match research.list_notes(session_id) {
                    Ok(notes) => match serde_json::to_value(&notes) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let id = match req.params.get("id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing id parameter"),
                };
                match research.get_note(id) {
                    Ok(Some(note)) => match serde_json::to_value(&note) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, "Note not found"),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let params: SaveNoteParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid save note params: {e}"))
                    }
                };

                let note = if let Some(ref note_id) = params.id {
                    match research.get_note(note_id) {
                        Ok(Some(mut existing)) => {
                            match existing.update(Some(params.title), params.content, params.tags) {
                                Ok(prev_ver) => {
                                    if let Err(e) = research.save_note(&existing) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    if let Err(e) = research.save_note_version(&prev_ver) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    let node = ArtifactLineageNode {
                                        artifact_path: format!("notes/{}.md", existing.id),
                                        version: existing.version,
                                        content_hash: existing.content_hash.clone(),
                                        produced_by_run_id: None,
                                        parent_version_hash: Some(prev_ver.content_hash),
                                        timestamp: existing.updated_at,
                                    };
                                    let _ = research.record_artifact_lineage(&node);
                                    existing
                                }
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            }
                        }
                        Ok(None) => {
                            match NoteRecord::new(
                                params.title,
                                params.content,
                                params.session_id,
                                params.task_id,
                                params.tags.unwrap_or_default(),
                            ) {
                                Ok(mut n) => {
                                    n.id = note_id.clone();
                                    if let Err(e) = research.save_note(&n) {
                                        return ApiResponse::error(req.id, e.to_string());
                                    }
                                    let node = ArtifactLineageNode {
                                        artifact_path: format!("notes/{}.md", n.id),
                                        version: 1,
                                        content_hash: n.content_hash.clone(),
                                        produced_by_run_id: None,
                                        parent_version_hash: None,
                                        timestamp: n.created_at,
                                    };
                                    let _ = research.record_artifact_lineage(&node);
                                    n
                                }
                                Err(e) => return ApiResponse::error(req.id, e.to_string()),
                            }
                        }
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                } else {
                    match NoteRecord::new(
                        params.title,
                        params.content,
                        params.session_id,
                        params.task_id,
                        params.tags.unwrap_or_default(),
                    ) {
                        Ok(n) => {
                            if let Err(e) = research.save_note(&n) {
                                return ApiResponse::error(req.id, e.to_string());
                            }
                            let node = ArtifactLineageNode {
                                artifact_path: format!("notes/{}.md", n.id),
                                version: 1,
                                content_hash: n.content_hash.clone(),
                                produced_by_run_id: None,
                                parent_version_hash: None,
                                timestamp: n.created_at,
                            };
                            let _ = research.record_artifact_lineage(&node);
                            n
                        }
                        Err(e) => return ApiResponse::error(req.id, e.to_string()),
                    }
                };

                match serde_json::to_value(&note) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTES_HISTORY => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let note_id = match req.params.get("note_id").and_then(|v| v.as_str()) {
                    Some(i) => i,
                    None => return ApiResponse::error(req.id, "Missing note_id parameter"),
                };
                match research.list_note_versions(note_id) {
                    Ok(vers) => match serde_json::to_value(&vers) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_LIST => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_notebook_cells(session_id) {
                    Ok(cells) => match serde_json::to_value(&cells) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_CELLS_SAVE => {
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let mut cells: Vec<NotebookCell> = match req
                    .params
                    .get("cells")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                {
                    Some(c) => c,
                    None => {
                        return ApiResponse::error(req.id, "Missing or invalid cells parameter")
                    }
                };
                if cells.iter().any(|cell| cell.session_id != session_id) {
                    return ApiResponse::error(
                        req.id,
                        "Notebook cell session_id must match the requested session_id",
                    );
                }
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                if let Some(kernel) = self.python_kernel.as_ref() {
                    let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                    match kernel
                        .get_status_scoped(
                            session_id,
                            notebook_id,
                            Some(research.as_ref()
                                as &(dyn custos_core::contracts::storage::NotebookStoragePort
                                      + Send
                                      + Sync)),
                        )
                        .await
                    {
                        Ok(state) if state.status == custos_domain::KernelStatus::Busy => {
                            return ApiResponse::error(
                                req.id,
                                "Wait for notebook execution or interrupt it before changing cells",
                            )
                        }
                        Err(error) => return ApiResponse::error(req.id, error.to_string()),
                        _ => {}
                    }
                }
                let stored = match research.list_notebook_cells(session_id) {
                    Ok(cells) => cells,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                for cell in &mut cells {
                    if let Some(previous) = stored.iter().find(|saved| saved.id == cell.id) {
                        if previous.source == cell.source && previous.cell_type == cell.cell_type {
                            cell.execution_count = previous.execution_count;
                            cell.status = previous.status;
                            cell.stdout = previous.stdout.clone();
                            cell.stderr = previous.stderr.clone();
                            cell.output_image = previous.output_image.clone();
                            cell.wall_ms = previous.wall_ms;
                        } else {
                            cell.execution_count = None;
                            cell.status = CellExecutionStatus::Idle;
                            cell.stdout = None;
                            cell.stderr = None;
                            cell.output_image = None;
                            cell.wall_ms = None;
                        }
                        cell.epoch = previous.epoch;
                    } else {
                        cell.execution_count = None;
                        cell.status = CellExecutionStatus::Idle;
                        cell.stdout = None;
                        cell.stderr = None;
                        cell.output_image = None;
                        cell.wall_ms = None;
                        cell.epoch = 0;
                    }
                }
                match research.replace_notebook_cells(session_id, &cells) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "count": cells.len() }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_EXECUTE => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let mut params: ExecuteCellParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid execute cell params: {e}"),
                        )
                    }
                };
                let workspace_id = match params
                    .workspace_id
                    .as_deref()
                    .filter(|id| !id.trim().is_empty())
                {
                    Some(id) => id,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "Notebook execution requires a registered workspace_id",
                        )
                    }
                };
                let workspace = match self
                    .resolve_workspace_for_dispatch(&req.id, workspace_id)
                    .await
                {
                    Ok(workspace) => workspace,
                    Err(response) => return response,
                };
                let research = match self.research.as_ref() {
                    Some(repository) => repository,
                    None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
                };
                let saved = match research.list_notebook_cells(&params.session_id) {
                    Ok(cells) => cells,
                    Err(error) => return ApiResponse::error(req.id, error.to_string()),
                };
                match saved.iter().find(|cell| cell.id == params.cell_id) {
                    Some(cell)
                        if cell.source == params.code
                            && cell.cell_type == custos_domain::NotebookCellType::Code => {}
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Save the exact code cell in this session before executing it",
                        )
                    }
                }
                // cwd is resolved from the registered workspace, never from daemon process state.
                if params
                    .cwd
                    .as_deref()
                    .is_some_and(|cwd| cwd != workspace.path)
                {
                    return ApiResponse::error(
                        req.id,
                        "Notebook cwd must match the registered workspace",
                    );
                }
                params.cwd = Some(workspace.path);

                let repo_ref = self.research.as_deref().map(|r| {
                    r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync)
                });
                match kernel.execute_cell(params, repo_ref).await {
                    Ok(res) => match serde_json::to_value(&res) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_INTERRUPT => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                match kernel.interrupt_scoped(session_id, notebook_id).await {
                    Ok(interrupted) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "interrupted": interrupted }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_RESET => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let repo_ref = self.research.as_deref().map(|r| {
                    r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync)
                });
                match kernel.reset_scoped(session_id, notebook_id, repo_ref).await {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_NOTEBOOK_STATUS => {
                let kernel = match self.python_kernel.as_ref() {
                    Some(k) => k,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "PythonKernelCoordinator not configured on daemon",
                        )
                    }
                };
                let session_id = match req.params.get("session_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.trim().is_empty() => s,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty session_id parameter")
                    }
                };
                let notebook_id = req.params.get("notebook_id").and_then(|v| v.as_str());
                let repo_ref = self.research.as_deref().map(|r| {
                    r as &(dyn custos_core::contracts::storage::NotebookStoragePort + Send + Sync)
                });
                match kernel
                    .get_status_scoped(session_id, notebook_id, repo_ref)
                    .await
                {
                    Ok(state) => match serde_json::to_value(&state) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let target_type = req
                    .params
                    .get("target_type")
                    .and_then(|v| serde_json::from_value(v.clone()).ok());
                let target_id = req.params.get("target_id").and_then(|v| v.as_str());
                match research.list_reviews(target_type, target_id) {
                    Ok(reviews) => match serde_json::to_value(&reviews) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("review_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.get_review(id) {
                    Ok(Some(rev)) => match serde_json::to_value(&rev) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_RECORD => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let params: RecordReviewParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid record review params: {e}"),
                        )
                    }
                };
                let record = match ReviewerRecord::new(
                    params.target_type,
                    params.target_id,
                    params.reviewer,
                    params.method,
                    params.status,
                    params.evidence_summary,
                    params.evidence_digest,
                    params.findings.unwrap_or_default(),
                ) {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                if let Err(e) = research.save_review(&record) {
                    return ApiResponse::error(req.id, e.to_string());
                }
                match serde_json::to_value(&record) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_REVIEWS_MARK_STALE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("review_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => return ApiResponse::error(req.id, "Missing or empty id parameter"),
                };
                match research.mark_review_stale(id) {
                    Ok(()) => {
                        ApiResponse::success(req.id, serde_json::json!({ "marked_stale": true }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_LIST => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                match research.list_synthesis_proposals() {
                    Ok(list) => match serde_json::to_value(&list) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_GET => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let id = match req
                    .params
                    .get("id")
                    .or_else(|| req.params.get("proposal_id"))
                    .and_then(|v| v.as_str())
                {
                    Some(i) if !i.trim().is_empty() => i,
                    _ => {
                        return ApiResponse::error(req.id, "Missing or empty proposal id parameter")
                    }
                };
                match research.get_synthesis_proposal(id) {
                    Ok(Some(prop)) => match serde_json::to_value(&prop) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::success(req.id, serde_json::Value::Null),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_PROPOSALS_SAVE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let params: SaveSynthesisProposalParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid save proposal params: {e}"),
                        )
                    }
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = match research.list_reviews(None, None) {
                    Ok(r) => r,
                    Err(_) => Vec::new(),
                };

                let claims: Vec<ClaimHandoffSummary> = params
                    .claim_ids
                    .iter()
                    .filter_map(|cid| {
                        all_claims.iter().find(|c| &c.id == cid).map(|c| {
                            let has_fresh_review = all_reviews.iter().any(|r| {
                                r.target_id == c.id
                                    && r.is_fresh
                                    && r.status == ReviewStatus::Approved
                            });
                            ClaimHandoffSummary {
                                claim_id: c.id.clone(),
                                statement: c.statement.clone(),
                                level: c.level,
                                confidence_score: c.confidence_score,
                                has_fresh_review,
                                sealed_proof_uri: c.sealed_proof_uri.clone(),
                            }
                        })
                    })
                    .collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let recipes: Vec<RecipeHandoffSummary> = params
                    .recipe_ids
                    .iter()
                    .filter_map(|rid| {
                        all_recipes
                            .iter()
                            .find(|r| &r.id == rid)
                            .map(|r| RecipeHandoffSummary {
                                recipe_id: r.id.clone(),
                                name: r.name.clone(),
                                command: r.command.clone(),
                                inputs_count: r.inputs.len(),
                                outputs: r.outputs.clone(),
                            })
                    })
                    .collect();

                let mut proposal = match ResearchSynthesisProposal::new(
                    params.title,
                    params.summary,
                    claims,
                    recipes,
                    params.artifact_paths.unwrap_or_default(),
                    params.workspace_id,
                    params.target_branch,
                ) {
                    Ok(p) => p,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                if let Some(existing_id) = params.id {
                    proposal.id = existing_id;
                }

                if let Err(e) = research.save_synthesis_proposal(&proposal) {
                    return ApiResponse::error(req.id, e.to_string());
                }

                match serde_json::to_value(&proposal) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            METHOD_SYNTHESIS_HANDOFF_EXECUTE => {
                let research = match self.research.as_ref() {
                    Some(r) => r,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "ResearchRepository not configured on daemon",
                        )
                    }
                };
                let params: HandoffToCodingParams = match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid handoff params: {e}"))
                    }
                };

                let all_claims = match research.list_claims() {
                    Ok(c) => c,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };
                let all_reviews = match research.list_reviews(None, None) {
                    Ok(r) => r,
                    Err(_) => Vec::new(),
                };

                // Invariant: Verify all requested claim IDs actually exist in the repository
                for cid in &params.claim_ids {
                    if !all_claims.iter().any(|c| &c.id == cid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Claim '{cid}' not found in research repository"),
                        );
                    }
                }
                let selected_claims: Vec<&ResearchClaim> = params
                    .claim_ids
                    .iter()
                    .filter_map(|cid| all_claims.iter().find(|c| &c.id == cid))
                    .collect();

                let all_recipes = match research.list_recipes() {
                    Ok(r) => r,
                    Err(e) => return ApiResponse::error(req.id, e.to_string()),
                };

                // Invariant: Verify all requested recipe IDs actually exist in the repository
                for rid in &params.recipe_ids {
                    if !all_recipes.iter().any(|r| &r.id == rid) {
                        return ApiResponse::error(
                            req.id,
                            format!("Recipe '{rid}' not found in research repository"),
                        );
                    }
                }
                let selected_recipes: Vec<&Recipe> = params
                    .recipe_ids
                    .iter()
                    .filter_map(|rid| all_recipes.iter().find(|r| &r.id == rid))
                    .collect();

                // Fail-closed verification gate: EVERY claim must be grounded/verified
                if params.enforce_verification && !selected_claims.is_empty() {
                    let unverified = selected_claims.iter().find(|c| {
                        !(c.level >= ClaimGroundingLevel::L2Verified
                            || all_reviews.iter().any(|r| {
                                r.target_id == c.id
                                    && r.is_fresh
                                    && r.status == ReviewStatus::Approved
                            }))
                    });
                    if let Some(bad_claim) = unverified {
                        return ApiResponse::error(
                            req.id,
                            format!(
                                "Fail-closed verification gate: claim '{}' is ungrounded (L0/unverified) and lacks approved active verifier record or L2+ proof",
                                bad_claim.id
                            ),
                        );
                    }
                }

                let claim_summaries: Vec<ClaimHandoffSummary> = selected_claims
                    .iter()
                    .map(|c| {
                        let has_fresh_review = all_reviews.iter().any(|r| {
                            r.target_id == c.id && r.is_fresh && r.status == ReviewStatus::Approved
                        });
                        ClaimHandoffSummary {
                            claim_id: c.id.clone(),
                            statement: c.statement.clone(),
                            level: c.level,
                            confidence_score: c.confidence_score,
                            has_fresh_review,
                            sealed_proof_uri: c.sealed_proof_uri.clone(),
                        }
                    })
                    .collect();

                let recipe_summaries: Vec<RecipeHandoffSummary> = selected_recipes
                    .iter()
                    .map(|r| RecipeHandoffSummary {
                        recipe_id: r.id.clone(),
                        name: r.name.clone(),
                        command: r.command.clone(),
                        inputs_count: r.inputs.len(),
                        outputs: r.outputs.clone(),
                    })
                    .collect();

                let caveats = ResearchSynthesisProposal::generate_caveats(
                    &claim_summaries,
                    &recipe_summaries,
                );
                let mut created_task_ids = Vec::new();

                // 1. Create coding tasks for claims
                for claim in &selected_claims {
                    let cmd = CreateTask {
                        title: format!("[Research Claim] {}", claim.statement),
                        metadata: Some(serde_json::json!({
                            "source": "research_synthesis_handoff",
                            "proposal_id": params.proposal_id,
                            "claim_id": claim.id,
                            "claim_statement": claim.statement,
                            "claim_level": claim.level,
                            "workspace_id": params.workspace_id,
                            "target_branch": params.target_branch,
                            "caveats": caveats,
                        })),
                        contract: Some(TaskContract {
                            pack_id: "engineering_verification".into(),
                            name: format!("Verify: {}", claim.statement),
                            description: format!("Empirically implement and produce diff / test verification for claim: {}", claim.statement),
                            required_capabilities: vec!["code.files".into(), "code.changes".into(), "compute.pty".into()],
                            evidence_requirements: vec![
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::Diff,
                                    required: true,
                                },
                                custos_domain::task::ContractEvidence {
                                    kind: custos_domain::task::EvidenceKind::TestResult,
                                    required: true,
                                },
                            ],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to child task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to create task: {e}"),
                            );
                        }
                    }
                }

                // 2. Create coding tasks for recipes
                for recipe in &selected_recipes {
                    let cmd = CreateTask {
                        title: format!("[Recipe Reproduction] {}", recipe.name),
                        metadata: Some(serde_json::json!({
                            "source": "research_synthesis_handoff",
                            "proposal_id": params.proposal_id,
                            "recipe_id": recipe.id,
                            "recipe_name": recipe.name,
                            "recipe_command": recipe.command,
                            "workspace_id": params.workspace_id,
                            "target_branch": params.target_branch,
                        })),
                        contract: Some(TaskContract {
                            pack_id: "recipe_execution".into(),
                            name: format!("Execute: {}", recipe.name),
                            description: format!("Execute recipe command: {}", recipe.command),
                            required_capabilities: vec!["compute.pty".into()],
                            evidence_requirements: vec![custos_domain::task::ContractEvidence {
                                kind: custos_domain::task::EvidenceKind::TestResult,
                                required: true,
                            }],
                        }),
                    };
                    match self.task_service.execute_create(cmd).await {
                        Ok((task, _)) => created_task_ids.push(task.id),
                        Err(e) => {
                            // Rollback created tasks in this batch to prevent partial child tasks on failure (B5 Invariant)
                            for created_id in &created_task_ids {
                                let _ = self.task_service.execute_advance(AdvanceTask {
                                    task_id: created_id.clone(),
                                    next_status: custos_domain::task::TaskStatus::Failed,
                                    expected_epoch: 1,
                                    rationale: Some(format!("Handoff aborted due to recipe task creation failure: {e}")),
                                }).await;
                            }
                            return ApiResponse::error(
                                req.id,
                                format!("Failed to create recipe task: {e}"),
                            );
                        }
                    }
                }

                // If proposal_id is provided, mark it completed
                if let Some(ref prop_id) = params.proposal_id {
                    if let Ok(Some(mut prop)) = research.get_synthesis_proposal(prop_id) {
                        prop.mark_completed();
                        let _ = research.save_synthesis_proposal(&prop);
                    }
                }

                let handoff_res = HandoffToCodingResult {
                    handoff_id: custos_domain::new_id("handoff"),
                    proposal_id: params.proposal_id,
                    task_ids: created_task_ids,
                    workspace_id: params.workspace_id,
                    target_branch: params.target_branch,
                    verified_claims_count: selected_claims.len(),
                    converted_recipes_count: selected_recipes.len(),
                    caveats,
                    timestamp: chrono::Utc::now().timestamp(),
                };

                match serde_json::to_value(&handoff_res) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown method: {unknown}")),
        }
    }
}
