//! Combinatorially Symmetric Cross-Validation (CSCV) PBO Verifier
//! Reference: Bailey, Borwein, López de Prado, Zhu (2016)
//! "The Probability of Backtest Overfitting"

use async_trait::async_trait;
use custos_domain::quant::calculate_pbo_cscv;
use crate::verifier::{PackVerifier, VerificationContext, VerificationResult};

pub struct PboOverfittingVerifier;

#[async_trait]
impl PackVerifier for PboOverfittingVerifier {
    fn verifier_id(&self) -> &'static str {
        "pbo_overfitting_verifier"
    }

    fn description(&self) -> &'static str {
        "Validates that quantitative trading strategies pass Combinatorially Symmetric Cross-Validation (CSCV PBO <= 0.50)"
    }

    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult {
        // 1. Check if pre-computed PBO probability is supplied
        if let Some(prob) = ctx.metadata.get("pbo_probability").and_then(|v| v.as_f64()) {
            if !prob.is_finite() || prob < 0.0 || prob > 1.0 {
                return VerificationResult::Fail {
                    reason: format!(
                        "Invalid CSCV PBO probability '{prob}'. Probability must be a finite scalar in range [0.0, 1.0]."
                    ),
                };
            }
            if prob <= 0.50 {
                return VerificationResult::Pass {
                    detail: format!("CSCV PBO passed: probability={:.2}% <= 50.0% (robust)", prob * 100.0),
                };
            } else {
                return VerificationResult::Fail {
                    reason: format!(
                        "CSCV PBO failed: probability={:.2}% > 50.0%. Strategy exhibits high likelihood of backtest overfitting.",
                        prob * 100.0
                    ),
                };
            }
        }

        // 2. Compute from matrix of subperiod returns if provided
        if let Some(matrix_val) = ctx.metadata.get("matrix").or_else(|| ctx.metadata.get("subperiod_returns")) {
            if let Some(rows) = matrix_val.as_array() {
                let mut matrix: Vec<Vec<f64>> = Vec::new();
                for r in rows {
                    if let Some(vals) = r.as_array() {
                        let row_vec: Vec<f64> = vals.iter().filter_map(|v| v.as_f64()).collect();
                        if !row_vec.is_empty() {
                            matrix.push(row_vec);
                        }
                    }
                }

                if !matrix.is_empty() {
                    let pbo_res = match calculate_pbo_cscv(&matrix, 2000) {
                        Ok(res) => res,
                        Err(e) => {
                            return VerificationResult::Fail {
                                reason: format!("CSCV PBO calculation error: {e}"),
                            };
                        }
                    };

                    if pbo_res.is_overfitted {
                        return VerificationResult::Fail {
                            reason: format!(
                                "CSCV PBO verification failed: overfitting probability={:.2}% (median logit={:.3}). Overfitting detected across {} combinations.",
                                pbo_res.probability * 100.0,
                                pbo_res.median_logit,
                                pbo_res.samples
                            ),
                        };
                    } else {
                        return VerificationResult::Pass {
                            detail: format!(
                                "CSCV PBO passed: overfitting probability={:.2}% (median logit={:.3}). Strategy evaluated across {} sub-period combinations.",
                                pbo_res.probability * 100.0,
                                pbo_res.median_logit,
                                pbo_res.samples
                            ),
                        };
                    }
                }
            }
        }

        // 3. Fallback: Default to pass if no multi-trial matrix was required for this task
        VerificationResult::Pass {
            detail: "No multi-trial returns matrix evaluated; PBO verification skipped as non-applicable.".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pbo_verifier_precomputed_pass() {
        let verifier = PboOverfittingVerifier;
        let mut ctx = VerificationContext::new("test_task");
        ctx.metadata = serde_json::json!({
            "pbo_probability": 0.15
        });

        let res = verifier.verify(&ctx).await;
        match res {
            VerificationResult::Pass { detail } => {
                assert!(detail.contains("robust"));
            }
            VerificationResult::Fail { reason } => panic!("Expected pass, got fail: {reason}"),
            _ => panic!("Expected pass"),
        }
    }

    #[tokio::test]
    async fn test_pbo_verifier_precomputed_fail() {
        let verifier = PboOverfittingVerifier;
        let mut ctx = VerificationContext::new("test_task");
        ctx.metadata = serde_json::json!({
            "pbo_probability": 0.78
        });

        let res = verifier.verify(&ctx).await;
        match res {
            VerificationResult::Fail { reason } => {
                assert!(reason.contains("overfitting"));
            }
            VerificationResult::Pass { detail } => panic!("Expected fail, got pass: {detail}"),
            _ => panic!("Expected fail"),
        }
    }

    #[tokio::test]
    async fn test_pbo_verifier_invalid_range_fail() {
        let verifier = PboOverfittingVerifier;
        let mut ctx = VerificationContext::new("test_task");
        ctx.metadata = serde_json::json!({
            "pbo_probability": -0.25
        });

        let res = verifier.verify(&ctx).await;
        match res {
            VerificationResult::Fail { reason } => {
                assert!(reason.contains("Invalid CSCV PBO probability"));
            }
            _ => panic!("Expected fail for negative probability"),
        }
    }
}
