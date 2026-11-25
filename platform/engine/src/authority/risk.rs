//! Risk Classification Engine
//!
//! Evaluates requested capabilities and actions to determine their risk class.

use custos_contracts::{Action, RiskClass};

pub struct RiskEvaluator;

impl RiskEvaluator {
    /// Classifies an action into its canonical RiskClass, enforcing a kernel floor
    /// so untrusted callers cannot downgrade dangerous operations to `Low`.
    pub fn classify_action(action: &Action) -> RiskClass {
        let lower = action.name.to_ascii_lowercase();
        let floor = if lower.contains("drop_database")
            || lower.contains("rm_rf")
            || lower.contains("format_disk")
            || lower.starts_with("dangerous_")
        {
            RiskClass::Critical
        } else if lower == "run_shell"
            || lower == "quant_order_propose"
            || lower == "quant_emergency_kill_switch"
        {
            RiskClass::High
        } else {
            RiskClass::Low
        };
        let rank = |r: RiskClass| match r {
            RiskClass::Low => 0u8,
            RiskClass::Medium => 1,
            RiskClass::High => 2,
            RiskClass::Critical => 3,
        };
        if rank(floor) > rank(action.risk_level) {
            floor
        } else {
            action.risk_level
        }
    }

    /// Determines if an action requires human approval based on risk class
    pub fn requires_human_approval(risk_class: RiskClass) -> bool {
        matches!(risk_class, RiskClass::High | RiskClass::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_contracts::RiskLevel;

    #[test]
    fn test_risk_classification() {
        let action = Action::new(
            "act_1".into(),
            "write_file".into(),
            "/path".into(),
            serde_json::json!({}),
            RiskLevel::Critical,
        );
        let risk = RiskEvaluator::classify_action(&action);
        assert_eq!(risk, RiskClass::Critical);
        assert!(RiskEvaluator::requires_human_approval(risk));
    }
}
