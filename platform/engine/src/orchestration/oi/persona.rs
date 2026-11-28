//! Persona Dispatcher & Trust Guards:
//! - `WhoCanAskGuard`: inbound admission (`WhoCanAskPolicy`)
//! - `ActsForTrustGuard`: outbound delegation (`ActsForTrustPolicy`)
//! - `PersonaDispatcher`: delegates planning to `&dyn OiPlannerPort` (DIP)

use custos_contracts::authority::action::RiskLevel;
use custos_contracts::oi::{AgentPersonaProfile, CallerIdentity};
use custos_contracts::{DecisionSnapshot, DomainError, OiPlannerPort, StrategyProposal, WorkbenchId};

/// Effective authority granted to a persona run after clamping against `ActsForTrustPolicy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClampedAuthority {
    pub effective_capabilities: Vec<String>,
    pub requires_biometric_approval: bool,
}

/// Inbound admission guard checking `WhoCanAskPolicy`.
pub struct WhoCanAskGuard;

impl WhoCanAskGuard {
    pub fn check(
        profile: &AgentPersonaProfile,
        caller: &CallerIdentity,
    ) -> Result<(), DomainError> {
        if !profile.enabled {
            return Err(DomainError::Unauthorized(format!(
                "Persona '{}' is disabled",
                profile.id
            )));
        }
        if !profile.allowed_triggers.allows_caller(caller) {
            return Err(DomainError::Unauthorized(format!(
                "Sender '{}' is not permitted by WhoCanAskPolicy for persona '{}'",
                caller.sender_id, profile.id
            )));
        }
        Ok(())
    }
}

/// Outbound authority guard enforcing `ActsForTrustPolicy`.
pub struct ActsForTrustGuard;

impl ActsForTrustGuard {
    pub fn evaluate(
        profile: &AgentPersonaProfile,
        caller: &CallerIdentity,
        requested_workbench: &WorkbenchId,
        requested_capabilities: &[String],
        requested_risk: RiskLevel,
    ) -> Result<ClampedAuthority, DomainError> {
        if caller.scope == "mobile" && !profile.acts_for.allow_mobile_dispatch {
            return Err(DomainError::Unauthorized(format!(
                "Persona '{}' does not allow mobile dispatch",
                profile.id
            )));
        }
        if !profile
            .acts_for
            .allowed_workbenches
            .contains(requested_workbench)
        {
            return Err(DomainError::Unauthorized(format!(
                "Workbench '{}' is outside ActsForTrustPolicy for persona '{}'",
                requested_workbench.as_str(),
                profile.id
            )));
        }

        let effective_capabilities = if profile.acts_for.allowed_capabilities.is_empty() {
            requested_capabilities.to_vec()
        } else {
            requested_capabilities
                .iter()
                .filter(|cap| profile.acts_for.allowed_capabilities.contains(cap))
                .cloned()
                .collect()
        };

        let requires_biometric_approval =
            risk_rank(requested_risk) > risk_rank(profile.acts_for.max_autonomous_risk)
                || (profile.acts_for.require_biometric_for_mutations
                    && risk_rank(requested_risk) >= risk_rank(RiskLevel::Medium));

        Ok(ClampedAuthority {
            effective_capabilities,
            requires_biometric_approval,
        })
    }
}

fn risk_rank(r: RiskLevel) -> u8 {
    match r {
        RiskLevel::Low => 0,
        RiskLevel::Medium => 1,
        RiskLevel::High => 2,
        RiskLevel::Critical => 3,
    }
}

pub struct PersonaDispatcher;

impl PersonaDispatcher {
    pub async fn dispatch(
        profile: &AgentPersonaProfile,
        caller: &CallerIdentity,
        snapshot: &DecisionSnapshot,
        oi: &dyn OiPlannerPort,
    ) -> Result<StrategyProposal, DomainError> {
        WhoCanAskGuard::check(profile, caller)?;
        oi.plan(snapshot).await
    }

    pub async fn plan_for_persona(
        profile: &AgentPersonaProfile,
        caller: &CallerIdentity,
        requested_workbench: &WorkbenchId,
        requested_risk: RiskLevel,
        snapshot: &DecisionSnapshot,
        oi: &dyn OiPlannerPort,
    ) -> Result<(StrategyProposal, ClampedAuthority), DomainError> {
        WhoCanAskGuard::check(profile, caller)?;
        let authority = ActsForTrustGuard::evaluate(
            profile,
            caller,
            requested_workbench,
            &snapshot.required_capabilities,
            requested_risk,
        )?;
        let mut clamped_snapshot = snapshot.clone();
        clamped_snapshot.required_capabilities = authority.effective_capabilities.clone();
        let proposal = oi.plan(&clamped_snapshot).await?;
        Ok((proposal, authority))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::oi::OiEngine;
    use custos_contracts::oi::{ActsForTrustPolicy, MemoryScopeId, WhoCanAskPolicy};

    fn sample_profile(policy: WhoCanAskPolicy) -> AgentPersonaProfile {
        AgentPersonaProfile {
            id: "persona-coder".into(),
            name: "Coder".into(),
            role: "Software Engineer".into(),
            default_workbench: WorkbenchId::new(WorkbenchId::CODING),
            memory_scope: MemoryScopeId::new("persona:coder"),
            allowed_triggers: policy,
            acts_for: ActsForTrustPolicy {
                allowed_workbenches: vec![WorkbenchId::new(WorkbenchId::CODING)],
                allowed_capabilities: vec!["fs_read".into()],
                max_autonomous_risk: RiskLevel::Low,
                require_biometric_for_mutations: true,
                allow_mobile_dispatch: true,
            },
            sovereign_instructions: String::new(),
            look_around_job_id: None,
            max_up_next_items: 5,
            enabled: true,
        }
    }

    fn owner_caller() -> CallerIdentity {
        CallerIdentity {
            sender_id: "owner".into(),
            scope: "owner_desktop".into(),
            is_paired_owner: true,
        }
    }

    fn external_caller(sender_id: &str) -> CallerIdentity {
        CallerIdentity {
            sender_id: sender_id.into(),
            scope: "external".into(),
            is_paired_owner: false,
        }
    }

    #[tokio::test]
    async fn owner_only_rejects_non_owner_and_allows_owner() {
        let oi = OiEngine::new();
        let snapshot = DecisionSnapshot::new("task-1");
        let owner_only = sample_profile(WhoCanAskPolicy::OwnerOnly);

        let err = PersonaDispatcher::dispatch(
            &owner_only,
            &external_caller("slack:user1"),
            &snapshot,
            &oi,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::Unauthorized(_)));

        let ok = PersonaDispatcher::dispatch(&owner_only, &owner_caller(), &snapshot, &oi).await;
        assert!(ok.is_ok());

        let allow_list = sample_profile(WhoCanAskPolicy::AllowList(vec!["slack:ops".into()]));
        assert!(
            PersonaDispatcher::dispatch(&allow_list, &external_caller("slack:ops"), &snapshot, &oi)
                .await
                .is_ok()
        );
        assert!(
            PersonaDispatcher::dispatch(&allow_list, &external_caller("slack:other"), &snapshot, &oi)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn acts_for_trust_guard_clamps_capabilities_and_flags_biometric() {
        let oi = OiEngine::new();
        let mut snapshot = DecisionSnapshot::new("task-2");
        snapshot.required_capabilities = vec!["fs_read".into(), "shell_exec".into()];
        let profile = sample_profile(WhoCanAskPolicy::OwnerOnly);

        let (_proposal, auth) = PersonaDispatcher::plan_for_persona(
            &profile,
            &owner_caller(),
            &WorkbenchId::new(WorkbenchId::CODING),
            RiskLevel::High,
            &snapshot,
            &oi,
        )
        .await
        .expect("plan_for_persona");

        assert_eq!(auth.effective_capabilities, vec!["fs_read".to_string()]);
        assert!(auth.requires_biometric_approval);

        let wrong_wb = PersonaDispatcher::plan_for_persona(
            &profile,
            &owner_caller(),
            &WorkbenchId::new(WorkbenchId::QUANT),
            RiskLevel::Low,
            &snapshot,
            &oi,
        )
        .await
        .unwrap_err();
        assert!(matches!(wrong_wb, DomainError::Unauthorized(_)));
    }
}
