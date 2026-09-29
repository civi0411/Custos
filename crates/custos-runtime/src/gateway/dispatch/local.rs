use anyhow::Result;
use custos_cognitive::RouteDecision;

pub struct LocalDispatcher;

impl LocalDispatcher {
    pub async fn dispatch(decision: &RouteDecision, prompt: &str) -> Result<String> {
        tracing::info!(
            tier = ?decision.tier,
            provider = %decision.provider_id,
            model = %decision.model_id,
            "Dispatching locally"
        );

        match decision.tier {
            custos_cognitive::ReasoningTier::TierZero => {
                // System 0 deterministic execution
                Ok(format!(
                    "[Tier 0 Deterministic Execution] Processed: {}",
                    prompt
                ))
            }
            _ => {
                // System 1 local/fast response
                Ok(format!(
                    "[{}/{}] Response to: {}",
                    decision.provider_id, decision.model_id, prompt
                ))
            }
        }
    }
}
