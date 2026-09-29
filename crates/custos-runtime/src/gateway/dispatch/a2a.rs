use anyhow::{anyhow, Result};
use crate::cognitive::RouteDecision;

pub struct A2ADispatcher;

impl A2ADispatcher {
    pub async fn dispatch(decision: &RouteDecision, prompt: &str) -> Result<String> {
        let Some(target) = &decision.a2a_target else {
            return Err(anyhow!("No A2A target configured in RouteDecision"));
        };

        tracing::info!(
            agent_id = %target.agent_id,
            mode = ?target.delegation_mode,
            trust = %target.trust_score,
            "Dispatching via A2A protocol"
        );

        Ok(format!(
            "[A2A Peer: {} (trust: {:.2})] Result for: {}",
            target.agent_id, target.trust_score, prompt
        ))
    }
}
