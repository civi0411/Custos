use crate::human_gate::{HumanGate, HumanGateOutcome};
use anyhow::Result;
use custos_core_domain::{RiskLevel, SessionId, TaskId};
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
pub enum ApprovalDecision {
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
    ) -> ApprovalDecision {
        if matches!(risk, RiskLevel::Critical) && self.block_critical_risk {
            return ApprovalDecision::Blocked {
                reason: "Critical risk actions are permanently blocked by policy".into(),
            };
        }

        if has_shell_exec {
            return ApprovalDecision::RequiresHuman(ApprovalKind::CodeExecution);
        }

        if has_file_write {
            return ApprovalDecision::RequiresHuman(ApprovalKind::StateMutation);
        }

        match risk {
            RiskLevel::Low => {
                if self.auto_approve_low_risk {
                    ApprovalDecision::AutoApproved
                } else {
                    ApprovalDecision::RequiresHuman(ApprovalKind::ToolExecution)
                }
            }
            RiskLevel::Medium => ApprovalDecision::RequiresHuman(ApprovalKind::ToolExecution),
            RiskLevel::High | RiskLevel::Critical => {
                ApprovalDecision::RequiresHuman(ApprovalKind::HighRiskAction)
            }
        }
    }
}

pub struct ApprovalRouter {
    policy: ApprovalPolicy,
    human_gate: HumanGate,
}

impl ApprovalRouter {
    pub fn new(policy: ApprovalPolicy, human_gate: HumanGate) -> Self {
        Self { policy, human_gate }
    }

    pub fn evaluate_action(
        &self,
        risk: RiskLevel,
        has_file_write: bool,
        has_shell_exec: bool,
    ) -> ApprovalDecision {
        self.policy.evaluate(risk, has_file_write, has_shell_exec)
    }

    pub async fn gate_if_required(
        &self,
        session_id: SessionId,
        task_id: Option<TaskId>,
        action_name: &str,
        risk: RiskLevel,
        has_file_write: bool,
        has_shell_exec: bool,
    ) -> Result<bool> {
        let decision = self.evaluate_action(risk, has_file_write, has_shell_exec);
        match decision {
            ApprovalDecision::AutoApproved => Ok(true),
            ApprovalDecision::Blocked { reason } => {
                tracing::warn!(action = %action_name, reason = %reason, "Action blocked by policy");
                Ok(false)
            }
            ApprovalDecision::RequiresHuman(kind) => {
                let msg = format!(
                    "Action '{}' requires human approval ({:?}, Risk: {:?})",
                    action_name, kind, risk
                );
                let outcome = self
                    .human_gate
                    .request_and_wait(session_id, task_id, msg, None, self.policy.default_timeout)
                    .await?;
                Ok(matches!(outcome, HumanGateOutcome::Approved(_)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_evaluations() {
        let policy = ApprovalPolicy::default();

        // Low risk without write/exec -> auto approved
        assert_eq!(
            policy.evaluate(RiskLevel::Low, false, false),
            ApprovalDecision::AutoApproved
        );

        // Low risk with file write -> RequiresHuman(StateMutation)
        assert_eq!(
            policy.evaluate(RiskLevel::Low, true, false),
            ApprovalDecision::RequiresHuman(ApprovalKind::StateMutation)
        );

        // Low risk with shell exec -> RequiresHuman(CodeExecution)
        assert_eq!(
            policy.evaluate(RiskLevel::Low, false, true),
            ApprovalDecision::RequiresHuman(ApprovalKind::CodeExecution)
        );

        // High risk -> RequiresHuman(HighRiskAction)
        assert_eq!(
            policy.evaluate(RiskLevel::High, false, false),
            ApprovalDecision::RequiresHuman(ApprovalKind::HighRiskAction)
        );
    }
}
