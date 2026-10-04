use async_trait::async_trait;
use custos_domain::DomainError;
use custos_domain::oi::{WorkerResult, WorkerStatus};
use super::graph_runtime::StepExecutor;

pub struct WorkerExecutor;

impl WorkerExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorkerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StepExecutor for WorkerExecutor {
    async fn execute_step(
        &self,
        node_id: &str,
        action_type: &str,
        inputs: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        if action_type != "execute_worker" {
            return Err(DomainError::Validation(format!("Unsupported action type: {}", action_type)));
        }

        let role = inputs.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let harness = inputs.get("harness_id").and_then(|v| v.as_str()).unwrap_or("unknown");
        
        let result = WorkerResult {
            packet_id: custos_domain::new_id("wpk"),
            status: WorkerStatus::Sufficient,
            payload: format!("Simulated execution of node {} by {} ({})", node_id, role, harness),
            tokens_used: 100,
            cost_usd: 0.01,
            evidence_references: vec![],
        };

        Ok(serde_json::to_value(result).map_err(|e| DomainError::Validation(e.to_string()))?)
    }
}
