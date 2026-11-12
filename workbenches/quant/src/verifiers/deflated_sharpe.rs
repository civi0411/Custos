//! Deflated Sharpe Ratio Verifier
//! Reference: Bailey & de Prado (2016) "The Deflated Sharpe Ratio"

use async_trait::async_trait;
use custos_core::contracts::pack::{PackVerifier, VerificationContext, VerificationResult};
use crate::domain::calculate_dsr;

pub struct DeflatedSharpeVerifier;

#[async_trait]
impl PackVerifier for DeflatedSharpeVerifier {
    fn verifier_id(&self) -> &'static str {
        "deflated_sharpe_verifier"
    }

    fn description(&self) -> &'static str {
        "Validates that algorithmic strategies demonstrate statistically significant Deflated Sharpe Ratio (DSR > 0.5)"
    }

    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult {
        let sharpe = ctx
            .metadata
            .get("annualized_sharpe")
            .or_else(|| ctx.metadata.get("sharpe_ratio"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let dsr = if let Some(d) = ctx.metadata.get("dsr_value").and_then(|v| v.as_f64()) {
            d
        } else {
            let trials = ctx
                .metadata
                .get("trials_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(1) as usize;
            let sample_len = ctx
                .metadata
                .get("sample_length")
                .and_then(|v| v.as_u64())
                .unwrap_or(252) as usize;
            let skewness = ctx
                .metadata
                .get("skewness")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let kurtosis = ctx
                .metadata
                .get("kurtosis")
                .and_then(|v| v.as_f64())
                .unwrap_or(3.0);
            let variance = ctx
                .metadata
                .get("variance_of_trials")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            calculate_dsr(sharpe, trials, sample_len, skewness, kurtosis, variance)
        };

        // DSR threshold: must be > 0.5 (better than random luck after multiple testing)
        if dsr > 0.5 && sharpe > 0.0 {
            VerificationResult::Pass {
                detail: format!("DSR/Sharpe passed: sharpe={sharpe:.3}, dsr={dsr:.4}"),
            }
        } else {
            VerificationResult::Fail {
                reason: format!(
                    "DSR verification failed: sharpe={sharpe:.3}, dsr={dsr:.4}. \
                     Strategy must show statistically significant positive returns (DSR > 0.50) after multiple testing correction."
                ),
            }
        }
    }
}
