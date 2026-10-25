//! Research artifacts, lineage, annotations, and notes use cases.

use crate::api::*;
use base64::Engine;
use custos_domain::{
    AnnotationStatus, AnnotationTarget, ArtifactLineageNode, NoteRecord,
};

impl LocalApiDispatcher {
    pub(in crate::api) async fn handle_research_artifacts(
        &self,
        req: ApiRequest,
    ) -> ApiResponse {
        match req.method.as_str() {
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
                    AnnotationTarget::Text {
                        start_offset,
                        end_offset,
                        exact_text,
                    } => {
                        start_offset < end_offset
                            && !exact_text.is_empty()
                            && exact_text.len() <= 65_536
                    }
                    AnnotationTarget::ImagePin {
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
                    AnnotationTarget::DomElement { selector } => {
                        !selector.trim().is_empty() && selector.len() <= 4096
                    }
                };
                if !valid_target || anno.status != AnnotationStatus::Pending {
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
            unknown => ApiResponse::error(req.id, format!("Unknown research artifacts method: {unknown}")),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use custos_persistence::SqliteTaskStore;

    #[tokio::test]
    async fn artifacts_group_test_without_dispatcher() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.research();

        let initial_notes = repo.list_notes(None).unwrap();
        assert_eq!(initial_notes.len(), 0);

        let note = NoteRecord::new(
            "Hypothesis 1".to_string(),
            "Investigating multi-head attention".to_string(),
            None,
            None,
            vec!["attention".to_string()],
        )
        .unwrap();

        repo.save_note(&note).unwrap();
        let fetched = repo.get_note(&note.id).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "Hypothesis 1");
    }
}
