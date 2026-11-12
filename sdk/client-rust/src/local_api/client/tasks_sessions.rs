use super::LocalApiClient;
use crate::local_api::envelope::ApiRequest;
use crate::local_api::methods::*;
use crate::local_api::params::*;
use custos_domain::{
    Session, SessionJournalEntry, Task, TaskContract, TaskStatus, VerificationClaim,
};
use serde::Deserialize;

impl LocalApiClient {
    pub async fn create_task(
        &self,
        req_id: &str,
        title: &str,
        contract: Option<TaskContract>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CreateTaskRequest {
            title: title.to_string(),
            contract,
            metadata,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn get_task(&self, req_id: &str, task_id: &str) -> Result<Task, String> {
        let params = serde_json::json!({ "task_id": task_id });
        let req = ApiRequest::new(req_id, METHOD_TASKS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn cancel_task(
        &self,
        req_id: &str,
        task_id: &str,
        reason: Option<String>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CancelTaskRequest {
            task_id: task_id.to_string(),
            reason,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_CANCEL, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn advance_task(
        &self,
        req_id: &str,
        task_id: &str,
        target_status: TaskStatus,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(AdvanceTaskRequest {
            task_id: task_id.to_string(),
            target_status,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_ADVANCE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn complete_task(
        &self,
        req_id: &str,
        task_id: &str,
        summary: Option<String>,
    ) -> Result<Task, String> {
        self.complete_task_with_evidence(req_id, task_id, summary, Vec::new())
            .await
    }

    pub async fn complete_task_with_evidence(
        &self,
        req_id: &str,
        task_id: &str,
        summary: Option<String>,
        evidence_claims: Vec<VerificationClaim>,
    ) -> Result<Task, String> {
        let params = serde_json::to_value(CompleteTaskRequest {
            task_id: task_id.to_string(),
            summary,
            evidence_claims,
        })
        .map_err(|e| e.to_string())?;

        let req = ApiRequest::new(req_id, METHOD_TASKS_COMPLETE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn list_tasks(&self, req_id: &str) -> Result<Vec<Task>, String> {
        let req = ApiRequest::new(req_id, METHOD_TASKS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse tasks list: {e}"))
    }

    pub async fn list_spans(
        &self,
        req_id: &str,
        task_id: &str,
    ) -> Result<Vec<custos_domain::Span>, String> {
        let params = serde_json::json!({ "task_id": task_id });
        let req = ApiRequest::new(req_id, METHOD_TASKS_SPANS, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse spans list: {e}"))
    }

    pub async fn create_session(
        &self,
        req_id: &str,
        mode: Option<&str>,
    ) -> Result<Session, String> {
        let params = serde_json::json!({
            "mode": mode.unwrap_or("bare")
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_CREATE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Session: {e}"))
    }

    pub async fn get_session(&self, req_id: &str, session_id: &str) -> Result<Session, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_GET, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Session: {e}"))
    }

    pub async fn delete_session(&self, req_id: &str, session_id: &str) -> Result<bool, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_DELETE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        Ok(result
            .get("deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    pub async fn list_sessions(&self, req_id: &str) -> Result<Vec<Session>, String> {
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_LIST, serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse sessions list: {e}"))
    }

    pub async fn append_session_message(
        &self,
        req_id: &str,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "role": role,
            "content": content,
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_MESSAGE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn get_session_journal(
        &self,
        req_id: &str,
        session_id: &str,
    ) -> Result<Vec<SessionJournalEntry>, String> {
        let params = serde_json::json!({ "session_id": session_id });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_JOURNAL, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        #[derive(Deserialize)]
        struct JournalWrapper {
            entries: Vec<SessionJournalEntry>,
        }
        if let Ok(wrapper) = serde_json::from_value::<JournalWrapper>(result.clone()) {
            return Ok(wrapper.entries);
        }
        serde_json::from_value(result).map_err(|e| format!("Failed to parse journal entries: {e}"))
    }

    pub async fn attach_session(
        &self,
        req_id: &str,
        session_id: &str,
        task_id: &str,
    ) -> Result<(), String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "task_id": task_id,
        });
        let req = ApiRequest::new(req_id, METHOD_BRIDGE_ATTACH, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(())
    }

    pub async fn steer_session(
        &self,
        req_id: &str,
        session_id: &str,
        task_id: &str,
        message: &str,
    ) -> Result<serde_json::Value, String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "task_id": task_id,
            "message": message,
        });
        let req = ApiRequest::new(req_id, METHOD_BRIDGE_STEER, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        resp.result.ok_or_else(|| "Empty result in response".into())
    }

    pub async fn promote_session(
        &self,
        req_id: &str,
        session_id: &str,
        contract: TaskContract,
    ) -> Result<Task, String> {
        let params = serde_json::json!({
            "session_id": session_id,
            "contract": contract,
        });
        let req = ApiRequest::new(req_id, METHOD_SESSIONS_PROMOTE, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse Task: {e}"))
    }

    pub async fn explain_plan(
        &self,
        req_id: &str,
        request: ExplainPlanRequest,
    ) -> Result<custos_domain::oi::ExplainReport, String> {
        let params = serde_json::to_value(request).map_err(|e| e.to_string())?;
        let req = ApiRequest::new(req_id, METHOD_OI_EXPLAIN, params);
        let resp = self.transport.send_request(req).await?;

        if let Some(err) = resp.error {
            return Err(err);
        }

        let result = resp.result.ok_or("Empty result in response")?;
        serde_json::from_value(result).map_err(|e| format!("Failed to parse ExplainReport: {e}"))
    }
}
