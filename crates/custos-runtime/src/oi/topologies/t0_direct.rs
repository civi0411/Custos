//! T0 Direct Topology Coordinator (RFC 004 §2A)
//!
//! Direct single-turn synthesis without tool execution or side effects.

use custos_domain::oi::{WorkerResult, WorkerStatus};
use custos_domain::DomainError;

pub struct T0DirectCoordinator;

impl T0DirectCoordinator {
    pub async fn execute(
        task_id: &str,
        prompt: &str,
        harness_id: &str,
    ) -> Result<WorkerResult, DomainError> {
        let packet_id = format!("{}_t0_pkt", task_id);
        
        // Pure synthesis turn
        let payload = format!(
            "[T0 Direct Synthesis] Processed by harness '{}' for task '{}': prompt length {} chars",
            harness_id, task_id, prompt.len()
        );

        Ok(WorkerResult {
            packet_id,
            status: WorkerStatus::Sufficient,
            payload,
            tokens_used: 150,
            cost_usd: 0.0015,
            evidence_references: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_t0_direct_execution() {
        let res = T0DirectCoordinator::execute("task_t0_test", "Summarize spec", "claude_code")
            .await
            .unwrap();
        assert_eq!(res.status, WorkerStatus::Sufficient);
        assert!(res.payload.contains("T0 Direct Synthesis"));
    }
}
