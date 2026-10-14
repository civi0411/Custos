use crate::cognitive::RouteDecision;
use anyhow::{anyhow, Result};

pub struct ExternalDispatcher;

impl ExternalDispatcher {
    pub async fn dispatch(decision: &RouteDecision, prompt: &str) -> Result<String> {
        let _ = (decision, prompt);
        Err(anyhow!("No external provider dispatcher is connected to this gateway"))
    }
}
