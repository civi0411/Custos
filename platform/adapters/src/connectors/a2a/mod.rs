//! Agent-to-Agent (A2A) Client and Dispatcher
//!
//! Synthesized from upstream AgentGateway (`a2a/mod.rs`).
//! Wire schemas live in `custos_protocol::a2a` and are re-exported here.

pub use custos_protocol::a2a::*;
use serde_json::Value;

/// Lightweight A2A Client for sending task delegations and cancellations
pub struct A2aClient {
    http_client: reqwest::Client,
    default_timeout: std::time::Duration,
}

impl Default for A2aClient {
    fn default() -> Self {
        Self::new()
    }
}

impl A2aClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            default_timeout: std::time::Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Delegate a task to a peer A2A agent under SADE supervision
    pub async fn delegate_task(
        &self,
        endpoint: &str,
        task: &A2aTaskPayload,
        auth_header: Option<&str>,
    ) -> Result<A2aTaskPayload, custos_domain::DomainError> {
        let req_body = A2aJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(custos_domain::new_id("a2a_req")),
            method: "tasks/send".to_string(),
            params: serde_json::to_value(task).unwrap_or_default(),
        };

        let mut req = self.http_client.post(endpoint).json(&req_body);
        if let Some(token) = auth_header {
            req = req.header("Authorization", token);
        }

        let resp = req.send().await.map_err(|e| {
            custos_domain::DomainError::Validation(format!(
                "A2A request to '{endpoint}' failed: {e}"
            ))
        })?;

        if !resp.status().is_success() {
            return Err(custos_domain::DomainError::Validation(format!(
                "A2A peer returned HTTP {}",
                resp.status()
            )));
        }

        let rpc_resp: A2aJsonRpcResponse = resp.json().await.map_err(|e| {
            custos_domain::DomainError::Validation(format!(
                "Failed to parse A2A JSON-RPC response: {e}"
            ))
        })?;

        if let Some(err) = rpc_resp.error {
            return Err(custos_domain::DomainError::Validation(format!(
                "A2A delegation error (code {}): {}",
                err.code, err.message
            )));
        }

        let result_val = rpc_resp.result.unwrap_or_default();
        let payload: A2aTaskPayload = serde_json::from_value(result_val).map_err(|e| {
            custos_domain::DomainError::Validation(format!("Invalid A2A payload returned: {e}"))
        })?;

        Ok(payload)
    }

    /// Send a cancellation signal to an active A2A task delegation (Generation fencing)
    pub async fn cancel_task(
        &self,
        endpoint: &str,
        task_id: &str,
        reason: &str,
        auth_header: Option<&str>,
    ) -> Result<(), custos_domain::DomainError> {
        let req_body = A2aJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: serde_json::json!(custos_domain::new_id("a2a_cancel")),
            method: "tasks/cancel".to_string(),
            params: serde_json::json!({
                "taskId": task_id,
                "reason": reason
            }),
        };

        let mut req = self.http_client.post(endpoint).json(&req_body);
        if let Some(token) = auth_header {
            req = req.header("Authorization", token);
        }

        let resp = req.send().await.map_err(|e| {
            custos_domain::DomainError::Validation(format!("A2A cancel request failed: {e}"))
        })?;

        if !resp.status().is_success() {
            return Err(custos_domain::DomainError::Validation(format!(
                "A2A peer returned HTTP {} on cancel",
                resp.status()
            )));
        }

        Ok(())
    }
}

/// Dispatcher for standard A2A JSON-RPC methods
pub struct A2aDispatcher;

impl A2aDispatcher {
    /// Inspect an A2A JSON-RPC request and extract context ID if present
    pub fn extract_context_id(req: &A2aJsonRpcRequest) -> Option<String> {
        req.params
            .get("contextId")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }

    /// Build a successful A2A response
    pub fn success_response(id: Value, result: Value) -> A2aJsonRpcResponse {
        A2aJsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Build a method not found error response
    pub fn method_not_found(id: Value, method: &str) -> A2aJsonRpcResponse {
        A2aJsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(A2aJsonRpcError {
                code: -32601,
                message: format!("A2A Method not found: {}", method),
                data: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_a2a_dispatcher_method_not_found() {
        let resp = A2aDispatcher::method_not_found(json!(1), "unknown.method");
        assert_eq!(resp.error.unwrap().code, -32601);
    }
}
