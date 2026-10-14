use crate::cognitive::RouteDecision;
use anyhow::{anyhow, Result};

pub struct A2ADispatcher;

impl A2ADispatcher {
    pub async fn dispatch(decision: &RouteDecision, prompt: &str) -> Result<String> {
        let Some(target) = &decision.a2a_target else {
            return Err(anyhow!("No A2A target configured in RouteDecision"));
        };

        let _ = (target, prompt);
        Err(anyhow!("No A2A transport is connected to this gateway"))
    }
}
