//! Persona dispatcher sitting above `OiEngine`.
//!
//! Enforces `WhoCanAskPolicy` before delegating planning to `OiEngine::plan`.

use custos_contracts::oi::{AgentPersonaProfile, DecisionSnapshot, StrategyProposal};
use custos_contracts::{contracts::OiPlannerPort, DomainError};

use super::engine::OiEngine;

pub struct PersonaDispatcher;

impl PersonaDispatcher {
    pub async fn dispatch(
        profile: &AgentPersonaProfile,
        sender: &str,
        is_owner: bool,
        snapshot: &DecisionSnapshot,
        oi: &OiEngine,
    ) -> Result<StrategyProposal, DomainError> {
        if !profile.allowed_triggers.allows(sender, is_owner) {
            return Err(DomainError::Unauthorized(format!(
                "Sender '{}' is not permitted by WhoCanAskPolicy for persona '{}'",
                sender, profile.id
            )));
        }
        oi.plan(snapshot).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_contracts::oi::{PersonaWorkbench, WhoCanAskPolicy};

    fn sample_profile(policy: WhoCanAskPolicy) -> AgentPersonaProfile {
        AgentPersonaProfile {
            id: "persona-ops".into(),
            name: "Ops Colleague".into(),
            role: "Autonomous Reviewer".into(),
            default_workbench: PersonaWorkbench::Coding,
            memory_scope: "scope:ops".into(),
            allowed_triggers: policy,
            look_around_job_id: Some("job-look-around".into()),
        }
    }

    #[tokio::test]
    async fn persona_dispatcher_enforces_who_can_ask_and_delegates_to_oi() {
        let oi = OiEngine::new();
        let snapshot = DecisionSnapshot::new("task-persona-1");

        let owner_only = sample_profile(WhoCanAskPolicy::OwnerOnly);
        assert!(
            PersonaDispatcher::dispatch(&owner_only, "guest", false, &snapshot, &oi)
                .await
                .is_err()
        );
        assert!(
            PersonaDispatcher::dispatch(&owner_only, "owner", true, &snapshot, &oi)
                .await
                .is_ok()
        );

        let allow_list = sample_profile(WhoCanAskPolicy::AllowList(vec!["slack:ops".into()]));
        assert!(
            PersonaDispatcher::dispatch(&allow_list, "slack:ops", false, &snapshot, &oi)
                .await
                .is_ok()
        );
    }
}

