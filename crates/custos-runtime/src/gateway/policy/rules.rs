use custos_domain::RiskLevel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayPolicyRule {
    pub name: String,
    pub max_risk_allowed_without_human: RiskLevel,
    pub allow_remote_a2a: bool,
    pub enforce_token_cap: bool,
}

impl Default for GatewayPolicyRule {
    fn default() -> Self {
        Self {
            name: "default_strict".into(),
            max_risk_allowed_without_human: RiskLevel::Low,
            allow_remote_a2a: true,
            enforce_token_cap: true,
        }
    }
}
