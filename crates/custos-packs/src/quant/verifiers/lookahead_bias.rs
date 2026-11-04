//! Lookahead Bias Checker
//! Lookahead bias = strategy accidentally uses future info during historical simulation.

use async_trait::async_trait;
use custos_domain::quant::detect_lookahead_bias;
use crate::verifier::{PackVerifier, VerificationContext, VerificationResult};

pub struct LookaheadBiasChecker;

#[async_trait]
impl PackVerifier for LookaheadBiasChecker {
    fn verifier_id(&self) -> &'static str {
        "lookahead_bias_checker"
    }

    fn description(&self) -> &'static str {
        "Validates that alpha signals do not exhibit lookahead bias or future data leakage"
    }

    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult {
        // If formula or code is present in metadata, perform static syntax scan
        let formula_candidate = ctx
            .metadata
            .get("formula")
            .or_else(|| ctx.metadata.get("signal_formula"))
            .or_else(|| ctx.metadata.get("signal_hypothesis"))
            .and_then(|v| v.as_str());

        if let Some(formula) = formula_candidate {
            if let Err(e) = detect_lookahead_bias(formula) {
                return VerificationResult::Fail {
                    reason: format!("Lookahead bias audit rejected formula: {e}"),
                };
            }
        }

        let clean = ctx
            .metadata
            .get("lookahead_clean")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| formula_candidate.is_some());

        if clean {
            VerificationResult::Pass {
                detail: "Lookahead bias audit confirmed clean".into(),
            }
        } else {
            VerificationResult::Fail {
                reason: "Lookahead bias not confirmed clean. \
                         Provide a validated formula or set lookahead_clean=true."
                    .into(),
            }
        }
    }
}
