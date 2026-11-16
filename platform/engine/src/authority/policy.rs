//! Policy Evaluation Contracts & Evaluators
//!
//! Evaluates policy rules before granting permits.

use async_trait::async_trait;
use custos_domain::{Action, DomainError, RiskLevel};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalKind {
    ToolExecution,
    StateMutation,
    FinancialExpense,
    CodeExecution,
    HighRiskAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicyVerdict {
    AutoApproved,
    RequiresHuman(ApprovalKind),
    Blocked { reason: String },
}

#[derive(Debug, Clone)]
pub struct ApprovalPolicy {
    pub auto_approve_low_risk: bool,
    pub block_critical_risk: bool,
    pub default_timeout: Duration,
}

impl Default for ApprovalPolicy {
    fn default() -> Self {
        Self {
            auto_approve_low_risk: true,
            block_critical_risk: false,
            default_timeout: Duration::from_secs(300),
        }
    }
}

impl ApprovalPolicy {
    pub fn evaluate(
        &self,
        risk: RiskLevel,
        has_file_write: bool,
        has_shell_exec: bool,
    ) -> ApprovalPolicyVerdict {
        if matches!(risk, RiskLevel::Critical) && self.block_critical_risk {
            return ApprovalPolicyVerdict::Blocked {
                reason: "Critical risk actions are permanently blocked by policy".into(),
            };
        }

        if has_shell_exec {
            return ApprovalPolicyVerdict::RequiresHuman(ApprovalKind::CodeExecution);
        }

        if has_file_write {
            return ApprovalPolicyVerdict::RequiresHuman(ApprovalKind::StateMutation);
        }

        match risk {
            RiskLevel::Low => {
                if self.auto_approve_low_risk {
                    ApprovalPolicyVerdict::AutoApproved
                } else {
                    ApprovalPolicyVerdict::RequiresHuman(ApprovalKind::ToolExecution)
                }
            }
            RiskLevel::Medium => ApprovalPolicyVerdict::RequiresHuman(ApprovalKind::ToolExecution),
            RiskLevel::High | RiskLevel::Critical => {
                ApprovalPolicyVerdict::RequiresHuman(ApprovalKind::HighRiskAction)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    RequireApproval { reason: String },
    Deny { reason: String },
}

#[async_trait]
pub trait PolicyEvaluator: Send + Sync {
    async fn evaluate(&self, task_id: &str, action: &Action)
        -> Result<PolicyDecision, DomainError>;
}

/// Default policy evaluator based on action risk level
#[derive(Debug, Default)]
pub struct DefaultPolicyEvaluator;

#[async_trait]
impl PolicyEvaluator for DefaultPolicyEvaluator {
    async fn evaluate(
        &self,
        _task_id: &str,
        action: &Action,
    ) -> Result<PolicyDecision, DomainError> {
        match action.risk_level {
            custos_domain::RiskLevel::Low | custos_domain::RiskLevel::Medium => {
                Ok(PolicyDecision::Allow)
            }
            custos_domain::RiskLevel::High => Ok(PolicyDecision::RequireApproval {
                reason: format!(
                    "High-risk action '{}' requires explicit human confirmation",
                    action.name
                ),
            }),
            custos_domain::RiskLevel::Critical => Ok(PolicyDecision::Deny {
                reason: format!(
                    "Critical action '{}' is blocked by default policy",
                    action.name
                ),
            }),
        }
    }
}
