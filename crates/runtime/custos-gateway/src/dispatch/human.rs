use anyhow::Result;
use custos_cognitive::{HumanGate, HumanGateOutcome};
use custos_core_domain::{SessionId, TaskId};
use std::time::Duration;

pub struct HumanDispatcher {
    gate: HumanGate,
}

impl HumanDispatcher {
    pub fn new(gate: HumanGate) -> Self {
        Self { gate }
    }

    pub async fn dispatch(
        &self,
        session_id: SessionId,
        task_id: Option<TaskId>,
        message: String,
        timeout: Duration,
    ) -> Result<HumanGateOutcome> {
        self.gate
            .request_and_wait(session_id, task_id, message, None, timeout)
            .await
    }
}
