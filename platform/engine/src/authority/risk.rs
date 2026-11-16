//! Risk Classification Engine
//!
//! Evaluates requested capabilities and actions to determine their risk class.

use custos_domain::{Action, RiskClass};

pub struct RiskEvaluator;

impl RiskEvaluator {
    /// Classifies an action into its canonical RiskClass (unified with RiskLevel).
    pub fn classify_action(action: &Action) -> RiskClass {
        action.risk_level
    }

    /// Determines if an action requires human approval based on risk class
    pub fn requires_human_approval(risk_class: RiskClass) -> bool {
        matches!(risk_class, RiskClass::High | RiskClass::Critical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::RiskLevel;

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
