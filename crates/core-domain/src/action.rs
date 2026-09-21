//! Side-Effect Action Definitions & Risk Levels
//!
//! Every action that modifies the workspace or talks to the network is wrapped
//! in an Action subject to CapabilityGateway and PolicyEngine verification.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub name: String,
    pub target: String,
    pub parameters: serde_json::Value,
    pub risk_level: RiskLevel,
    pub evidence_required: bool,
}

impl Action {
    pub fn new(
        id: String,
        name: String,
        target: String,
        parameters: serde_json::Value,
        risk_level: RiskLevel,
    ) -> Self {
        let evidence_required = matches!(risk_level, RiskLevel::High | RiskLevel::Critical);
        Self {
            id,
            name,
            target,
            parameters,
            risk_level,
            evidence_required,
        }
    }
}
