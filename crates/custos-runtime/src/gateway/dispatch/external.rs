use crate::cognitive::RouteDecision;
use anyhow::Result;

pub struct ExternalDispatcher;

impl ExternalDispatcher {
    pub async fn dispatch(decision: &RouteDecision, prompt: &str) -> Result<String> {
        tracing::info!(
            provider = %decision.provider_id,
            model = %decision.model_id,
            "Dispatching to external provider / subprocess CLI"
        );

        Ok(format!(
            "[External Provider: {} / {}] Processed: {}",
            decision.provider_id, decision.model_id, prompt
        ))
    }
}
