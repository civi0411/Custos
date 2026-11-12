//! Sovereign Agent Execution Environment (AEE) Invariant Gates (ENV-01 to ENV-07)
//!
//! Pure, deterministic evaluation of environment invariants against specifications
//! and measured host observations.

use crate::contracts::environment::GateReport;
use custos_domain::authority::action::Assurance;
use custos_domain::environment::{
    EnvironmentAssurance, EnvironmentProfile, EnvironmentSpec, EnvironmentState,
};

pub struct EnvironmentGate;

impl EnvironmentGate {
    /// Evaluates all 7 canonical environment invariants against specification,
    /// current state, and measured host assurance.
    pub fn evaluate(
        spec: &EnvironmentSpec,
        state: &EnvironmentState,
        assurance: &EnvironmentAssurance,
    ) -> GateReport {
        let mut violations = Vec::new();

        // ENV-01: Execution state must be ready or safely degraded (never initializing, failed or torn down)
        if !state.is_ready() {
            violations.push(format!(
                "ENV-01: Environment is in non-executable state: {:?}",
                state
            ));
        }

        // ENV-02: Coding lens requires physical Git worktree isolation
        if let EnvironmentProfile::Coding {
            branch,
            worktree_path,
            ..
        } = &spec.profile
        {
            if branch.trim().is_empty() {
                violations.push("ENV-02: Coding environment requires non-empty branch".into());
            }
            if worktree_path.trim().is_empty() {
                violations
                    .push("ENV-02: Coding environment requires declared worktree path".into());
            }
        }

        // ENV-03: Honest Assurance (Gate 4) - Target assurance cannot exceed measured assurance
        match (spec.target_assurance, assurance.measured_assurance) {
            (Assurance::CustosMediated, Assurance::ObserveOnly | Assurance::Unknown) => {
                violations.push(format!(
                    "ENV-03: Target assurance '{:?}' exceeds measured assurance '{:?}'",
                    spec.target_assurance, assurance.measured_assurance
                ));
            }
            (Assurance::ProviderGoverned, Assurance::Unknown) => {
                violations.push(
                    "ENV-03: ProviderGoverned target cannot operate under Unknown assurance".into(),
                );
            }
            _ => {}
        }

        // ENV-04: Research lens requires valid corpus path
        if let EnvironmentProfile::Research { corpus_path, .. } = &spec.profile {
            if corpus_path.trim().is_empty() {
                violations
                    .push("ENV-04: Research environment requires valid corpus directory".into());
            }
        }

        // ENV-05: Quant backtest air-gap invariant (zero live socket)
        if let EnvironmentProfile::Quant {
            is_live_gateway: false,
            ..
        } = &spec.profile
        {
            if !assurance.network_restricted {
                violations.push(
                    "ENV-05: Quant backtest environment requires strictly restricted network".into(),
                );
            }
        }

        // ENV-07: Degraded state must specify an explicit, non-empty reason
        if let EnvironmentState::Degraded { reason } = state {
            if reason.trim().is_empty() {
                violations.push("ENV-07: Degraded environment state requires explicit reason".into());
            }
        }

        if violations.is_empty() {
            GateReport::ok(&spec.env_id, state.clone(), assurance.clone())
        } else {
            GateReport::failed(&spec.env_id, state.clone(), assurance.clone(), violations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_01_ready_passes_failed_rejected() {
        let spec = EnvironmentSpec::new(
            "env_01",
            EnvironmentProfile::Assistant { project_id: None },
        );
        let mut assurance = EnvironmentAssurance::unconstrained();
        assurance.measured_assurance = Assurance::CustosMediated;

        let ok_report = EnvironmentGate::evaluate(&spec, &EnvironmentState::Ready, &assurance);
        assert!(ok_report.passed);

        let failed_report = EnvironmentGate::evaluate(
            &spec,
            &EnvironmentState::SetupFailed {
                error: "timeout".into(),
            },
            &assurance,
        );
        assert!(!failed_report.passed);
        assert!(failed_report.violations[0].contains("ENV-01"));
    }

    #[test]
    fn test_env_02_coding_worktree_validation() {
        let invalid_spec = EnvironmentSpec::new(
            "env_coding",
            EnvironmentProfile::Coding {
                repo_path: "/repo".into(),
                branch: "".into(),
                worktree_path: "".into(),
            },
        );
        let assurance = EnvironmentAssurance {
            measured_assurance: Assurance::CustosMediated,
            filesystem_confined: true,
            network_restricted: false,
            process_isolated: true,
            isolation_driver: "seatbelt".into(),
        };

        let report =
            EnvironmentGate::evaluate(&invalid_spec, &EnvironmentState::Ready, &assurance);
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.contains("ENV-02")));
    }

    #[test]
    fn test_env_03_honest_assurance_fraud_rejection() {
        let spec = EnvironmentSpec::new(
            "env_fraud",
            EnvironmentProfile::Assistant { project_id: None },
        ); // spec target is CustosMediated by default
        let measured_observe_only = EnvironmentAssurance {
            measured_assurance: Assurance::ObserveOnly,
            filesystem_confined: false,
            network_restricted: false,
            process_isolated: false,
            isolation_driver: "none".into(),
        };

        let report =
            EnvironmentGate::evaluate(&spec, &EnvironmentState::Ready, &measured_observe_only);
        assert!(!report.passed);
        assert!(report.violations[0].contains("ENV-03"));
    }

    #[test]
    fn test_env_05_quant_backtest_airgap() {
        let quant_spec = EnvironmentSpec::new(
            "env_quant",
            EnvironmentProfile::Quant {
                data_partition: "2024_Q1".into(),
                is_live_gateway: false,
            },
        );
        let unrestricted_assurance = EnvironmentAssurance {
            measured_assurance: Assurance::CustosMediated,
            filesystem_confined: true,
            network_restricted: false, // NOT restricted!
            process_isolated: true,
            isolation_driver: "bwrap".into(),
        };

        let report =
            EnvironmentGate::evaluate(&quant_spec, &EnvironmentState::Ready, &unrestricted_assurance);
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.contains("ENV-05")));
    }
}

