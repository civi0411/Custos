use crate::cognitive::{ReasoningTier, RouteDecision};
use anyhow::{anyhow, Result};

pub struct LocalDispatcher;

impl LocalDispatcher {
    pub async fn dispatch(decision: &RouteDecision, prompt: &str) -> Result<String> {
        let _ = (decision, prompt, ReasoningTier::TierZero);
        Err(anyhow!("No local inference or deterministic executor is connected to this gateway"))
    }
}
