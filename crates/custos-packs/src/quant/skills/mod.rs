//! Quant Domain Pack Skills
//!
//! Provides algorithmic alpha discovery and risk simulation tools for quantitative strategies:
//! - Lookahead bias static analysis audit
//! - Deflated Sharpe Ratio (DSR) Extreme Value Theory calculation
//! - Combinatorially Symmetric Cross-Validation (CSCV) Probability of Backtest Overfitting (PBO)

use async_trait::async_trait;
use custos_domain::quant::{calculate_dsr, calculate_pbo_cscv, detect_lookahead_bias};
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, SkillRegistry,
};
use serde_json::json;
use std::sync::Arc;

pub struct QuantLookaheadAuditSkill;

#[async_trait]
impl Skill for QuantLookaheadAuditSkill {
    fn skill_id(&self) -> &'static str {
        "quant_lookahead_audit"
    }

    fn description(&self) -> &'static str {
        "Scans an alpha factor or trading rule formula for lookahead bias and future-data leakage"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(5_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let formula = ctx
            .args
            .get("formula")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("formula".to_string()))?;

        match detect_lookahead_bias(formula) {
            Ok(()) => Ok(SkillOutput::clean(
                self.skill_id(),
                json!({
                    "formula": formula,
                    "lookahead_clean": true,
                    "verdict": "Pass: Zero future-data leakage detected"
                }),
            )),
            Err(e) => Ok(SkillOutput::clean(
                self.skill_id(),
                json!({
                    "formula": formula,
                    "lookahead_clean": false,
                    "verdict": format!("Fail: {e}")
                }),
            )),
        }
    }
}

pub struct QuantDsrCalculateSkill;

#[async_trait]
impl Skill for QuantDsrCalculateSkill {
    fn skill_id(&self) -> &'static str {
        "quant_dsr_calculate"
    }

    fn description(&self) -> &'static str {
        "Calculates the Deflated Sharpe Ratio (DSR) using Extreme Value Theory to penalize multiple testing"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(5_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let sharpe = ctx
            .args
            .get("sharpe")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| SkillError::MissingArg("sharpe".to_string()))?;

        let trials_count = ctx
            .args
            .get("trials_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let sample_length = ctx
            .args
            .get("sample_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(252) as usize;

        let skewness = ctx.args.get("skewness").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let kurtosis = ctx.args.get("kurtosis").and_then(|v| v.as_f64()).unwrap_or(3.0);
        let variance = ctx
            .args
            .get("variance_of_trials")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        let dsr = calculate_dsr(
            sharpe,
            trials_count,
            sample_length,
            skewness,
            kurtosis,
            variance,
        );

        Ok(SkillOutput::clean(
            self.skill_id(),
            json!({
                "annualized_sharpe": sharpe,
                "deflated_sharpe_ratio": dsr,
                "trials_evaluated": trials_count,
                "sample_length": sample_length,
                "is_statistically_significant": dsr >= 0.95,
            }),
        ))
    }
}

pub struct QuantPboEvaluateSkill;

#[async_trait]
impl Skill for QuantPboEvaluateSkill {
    fn skill_id(&self) -> &'static str {
        "quant_pbo_evaluate"
    }

    fn description(&self) -> &'static str {
        "Computes Combinatorially Symmetric Cross-Validation (CSCV) to evaluate Probability of Backtest Overfitting"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(10_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let matrix_raw = ctx
            .args
            .get("returns_matrix")
            .and_then(|v| v.as_array())
            .ok_or_else(|| SkillError::MissingArg("returns_matrix".to_string()))?;

        let mut matrix = Vec::new();
        for row in matrix_raw {
            if let Some(arr) = row.as_array() {
                let row_f64: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
                if !row_f64.is_empty() {
                    matrix.push(row_f64);
                }
            }
        }

        let partitions = ctx
            .args
            .get("partitions")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as usize;

        match calculate_pbo_cscv(&matrix, partitions) {
            Ok(pbo) => Ok(SkillOutput::clean(
                self.skill_id(),
                json!({
                    "probability_of_overfitting": pbo.probability,
                    "combinations_evaluated": pbo.samples,
                    "median_logit": pbo.median_logit,
                    "is_overfitted": pbo.is_overfitted,
                }),
            )),
            Err(e) => Err(SkillError::Other(format!("PBO calculation failed: {e}"))),
        }
    }
}

pub fn register_quant_skills(registry: &mut SkillRegistry) {
    registry.register(Arc::new(QuantLookaheadAuditSkill));
    registry.register(Arc::new(QuantDsrCalculateSkill));
    registry.register(Arc::new(QuantPboEvaluateSkill));

    // Configure tier permissions for SystemTwo / quant pack
    registry.set_tier_permissions(
        "system_two",
        "quant",
        vec![
            "quant_lookahead_audit".into(),
            "quant_dsr_calculate".into(),
            "quant_pbo_evaluate".into(),
            "file_read".into(),
            "lexical_search".into(),
        ],
    );

    // Configure tier permissions for SystemOne / quant pack (read-only audit)
    registry.set_tier_permissions(
        "system_one",
        "quant",
        vec![
            "quant_lookahead_audit".into(),
            "quant_dsr_calculate".into(),
            "file_read".into(),
        ],
    );
}
