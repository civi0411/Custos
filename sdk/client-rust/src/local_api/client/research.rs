use super::LocalApiClient;
use crate::local_api::envelope::ApiRequest;
use crate::local_api::methods::*;
use crate::local_api::params::*;
use custos_domain::{
    ExecuteCellParams, ExecuteCellResult, NotebookCell, NotebookKernelState, RecordReviewParams,
    ReviewTargetType, ReviewerRecord,
};

impl LocalApiClient {
    pub async fn list_harnesses(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_protocol::harness::HarnessDescriptor>, String> {
        let req = ApiRequest::new(req_id, METHOD_HARNESS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse harnesses list: {e}"))
    }

    pub async fn get_harness(
        &self,
        req_id: &str,
        harness_id: &str,
    ) -> Result<custos_protocol::harness::HarnessDescriptor, String> {
        let params = serde_json::json!({ "harness_id": harness_id });
        let req = ApiRequest::new(req_id, METHOD_HARNESS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse harness descriptor: {e}"))
    }

    pub async fn run_native_harness(
        &self,
        req_id: &str,
        request: RunNativeHarnessRequest,
    ) -> Result<custos_protocol::harness::HarnessExecutionResult, String> {
        let params = serde_json::to_value(request).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_HARNESS_RUN_NATIVE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse run native result: {e}"))
    }

    pub async fn list_artifacts(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_domain::ArtifactSummary>, String> {
        let req = ApiRequest::new(req_id, METHOD_ARTIFACTS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse artifact list: {e}"))
    }

    pub async fn get_artifact(
        &self,
        req_id: &str,
        path: &str,
    ) -> Result<ArtifactDetailResponse, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_ARTIFACTS_GET,
            serde_json::json!({ "artifact_path": path }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse artifact detail: {e}"))
    }

    pub async fn get_artifact_lineage_graph(
        &self,
        req_id: &str,
        filter_path: Option<&str>,
    ) -> Result<custos_domain::ArtifactLineageGraph, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_ARTIFACTS_LINEAGE_GRAPH,
            serde_json::json!({ "artifact_path": filter_path }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse lineage graph: {e}"))
    }

    pub async fn list_notes(
        &self,
        req_id: &str,
        session_id: Option<&str>,
    ) -> Result<Vec<custos_protocol::research::NoteRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTES_LIST,
            serde_json::json!({ "session_id": session_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse notes list: {e}"))
    }

    pub async fn get_note(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<custos_protocol::research::NoteRecord>, String> {
        let req = ApiRequest::new(req_id, METHOD_NOTES_GET, serde_json::json!({ "id": id }));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse note: {e}"))
    }

    pub async fn save_note(
        &self,
        req_id: &str,
        params: SaveNoteParams,
    ) -> Result<custos_protocol::research::NoteRecord, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_NOTES_SAVE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse saved note: {e}"))
    }

    pub async fn list_note_versions(
        &self,
        req_id: &str,
        note_id: &str,
    ) -> Result<Vec<custos_protocol::research::NoteVersionRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_NOTES_HISTORY,
            serde_json::json!({ "note_id": note_id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse note versions: {e}"))
    }

    pub async fn list_notebook_cells(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<Vec<NotebookCell>, String> {
        self.list_notebook_cells_scoped(req_id, session_id, None)
            .await
    }

    pub async fn list_notebook_cells_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<Vec<NotebookCell>, String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_CELLS_LIST, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse notebook cells: {e}"))
    }

    pub async fn save_notebook_cells(
        &self,
        req_id: &str,
        session_id: &str,
        cells: Vec<NotebookCell>,
    ) -> Result<(), String> {
        self.save_notebook_cells_scoped(req_id, session_id, None, cells)
            .await
    }

    pub async fn save_notebook_cells_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
        cells: Vec<NotebookCell>,
    ) -> Result<(), String> {
        let mut params = serde_json::json!({ "session_id": session_id, "cells": cells });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_CELLS_SAVE, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn execute_notebook_cell(
        &self,
        req_id: &str,
        params: ExecuteCellParams,
    ) -> Result<ExecuteCellResult, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_EXECUTE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse execution result: {e}"))
    }

    pub async fn interrupt_notebook_kernel(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<(), String> {
        self.interrupt_notebook_kernel_scoped(req_id, session_id, None)
            .await
    }

    pub async fn interrupt_notebook_kernel_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<(), String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_INTERRUPT, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn reset_notebook_kernel(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<NotebookKernelState, String> {
        self.reset_notebook_kernel_scoped(req_id, session_id, None)
            .await
    }

    pub async fn reset_notebook_kernel_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_RESET, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse kernel state: {e}"))
    }

    pub async fn get_notebook_kernel_status(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<NotebookKernelState, String> {
        self.get_notebook_kernel_status_scoped(req_id, session_id, None)
            .await
    }

    pub async fn get_notebook_kernel_status_scoped(
        &self,
        req_id: &str,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, String> {
        let mut params = serde_json::json!({ "session_id": session_id });
        if let Some(nb) = notebook_id {
            params["notebook_id"] = serde_json::Value::String(nb.to_string());
        }
        let req = ApiRequest::new(req_id, METHOD_NOTEBOOK_STATUS, params);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse kernel state: {e}"))
    }

    pub async fn list_reviews(
        &self,
        req_id: &str,
        target_type: Option<ReviewTargetType>,
        target_id: Option<&str>,
    ) -> Result<Vec<ReviewerRecord>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_REVIEWS_LIST,
            serde_json::json!({
                "target_type": target_type,
                "target_id": target_id,
            }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse reviews: {e}"))
    }

    pub async fn get_review(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<ReviewerRecord>, String> {
        let req = ApiRequest::new(req_id, METHOD_REVIEWS_GET, serde_json::json!({ "id": id }));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse review: {e}"))
    }

    pub async fn record_review(
        &self,
        req_id: &str,
        params: RecordReviewParams,
    ) -> Result<ReviewerRecord, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_REVIEWS_RECORD, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse recorded review: {e}"))
    }

    pub async fn mark_review_stale(&self, req_id: &str, id: &str) -> Result<(), String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_REVIEWS_MARK_STALE,
            serde_json::json!({ "id": id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn list_synthesis_proposals(
        &self,
        req_id: &str,
    ) -> Result<Vec<custos_protocol::research::ResearchSynthesisProposal>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_SYNTHESIS_PROPOSALS_LIST,
            serde_json::json!({}),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse proposals: {e}"))
    }

    pub async fn get_synthesis_proposal(
        &self,
        req_id: &str,
        id: &str,
    ) -> Result<Option<custos_protocol::research::ResearchSynthesisProposal>, String> {
        let req = ApiRequest::new(
            req_id,
            METHOD_SYNTHESIS_PROPOSALS_GET,
            serde_json::json!({ "id": id }),
        );
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse proposal: {e}"))
    }

    pub async fn save_synthesis_proposal(
        &self,
        req_id: &str,
        params: custos_protocol::research::SaveSynthesisProposalParams,
    ) -> Result<custos_protocol::research::ResearchSynthesisProposal, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_SYNTHESIS_PROPOSALS_SAVE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse saved proposal: {e}"))
    }

    pub async fn execute_synthesis_handoff(
        &self,
        req_id: &str,
        params: custos_protocol::research::HandoffToCodingParams,
    ) -> Result<custos_protocol::research::HandoffToCodingResult, String> {
        let val = serde_json::to_value(params).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_SYNTHESIS_HANDOFF_EXECUTE, val);
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse handoff result: {e}"))
    }
}
