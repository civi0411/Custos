//! Measured Optimization Evaluation: S1/OI vs. Pinned Baseline (F11 / G12)
//!
//! Enforces empirical comparison invariants:
//! - Direct pinned baseline is the reference standard.
//! - Candidate optimization cannot claim cost reduction or quality preservation
//!   without statistically and empirically meeting pre-configured margins.
//! - Failures, human intervention time, and recovery costs must be included.
//! - Inconclusive results fall back to baseline default.

use serde::{Deserialize, Serialize};

/// Metric snapshot of an individual task execution run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExecutionMetrics {
    pub task_id: String,
    pub harness_id: String,
    pub success: bool,
    /// Score between 0.0 and 1.0 (fraction of criteria verified).
    pub quality_score: f64,
    pub tokens_used: u64,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub human_intervention_seconds: u64,
    pub attempts_count: usize,
    pub recovery_cost_usd: f64,
}

impl TaskExecutionMetrics {
    /// Computes the comprehensive economic cost including direct API spend,
    /// recovery attempts, and valued human intervention time.
    pub fn total_economic_cost(&self, human_hourly_rate_usd: f64) -> f64 {
        let human_cost = (self.human_intervention_seconds as f64 / 3600.0) * human_hourly_rate_usd;
        self.cost_usd + self.recovery_cost_usd + human_cost
    }
}

/// Configuration specifying acceptance margins and statistical thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonMarginConfig {
    /// Maximum allowed drop in quality score (e.g. 0.05 for 5% non-inferiority margin).
    pub quality_tolerance_margin: f64,
    /// Minimum required percentage cost reduction (e.g. 0.15 for 15% cheaper).
    pub cost_reduction_margin: f64,
    /// Minimum paired samples required before reaching a conclusive verdict.
    pub min_sample_size: usize,
    /// Valuation rate for human intervention ($/hour) to prevent hidden labor shifting.
    pub human_hourly_rate_usd: f64,
}

impl Default for ComparisonMarginConfig {
    fn default() -> Self {
        Self {
            quality_tolerance_margin: 0.05,
            cost_reduction_margin: 0.15,
            min_sample_size: 3,
            human_hourly_rate_usd: 60.0,
        }
    }
}

/// The definitive outcome of an optimization comparison.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OptimizationClaimOutcome {
    /// Verified: Quality is non-inferior and total economic cost meets or exceeds the reduction margin.
    QualityPreservedAndCheaper {
        quality_delta: f64,
        cost_savings_ratio: f64,
    },
    /// Rejected: Candidate failed criteria or dropped quality below the allowable tolerance margin.
    QualityInferior {
        baseline_quality: f64,
        candidate_quality: f64,
        quality_delta: f64,
    },
    /// Rejected: Quality is preserved, but total cost (including retries/human time/recovery)
    /// did not meet the required reduction margin.
    NotCheaper {
        baseline_cost: f64,
        candidate_cost: f64,
        cost_savings_ratio: f64,
    },
    /// Inconclusive: Sample size is insufficient. Pinned baseline remains default.
    Inconclusive {
        samples_observed: usize,
        samples_required: usize,
    },
}

impl OptimizationClaimOutcome {
    pub fn is_admissible_optimization(&self) -> bool {
        matches!(self, Self::QualityPreservedAndCheaper { .. })
    }
}

/// Evaluation report summarizing a paired benchmark dataset comparing candidate vs. pinned baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizationEvaluationReport {
    pub baseline_harness: String,
    pub candidate_harness: String,
    pub sample_count: usize,
    pub avg_baseline_quality: f64,
    pub avg_candidate_quality: f64,
    pub avg_baseline_total_cost: f64,
    pub avg_candidate_total_cost: f64,
    pub avg_baseline_latency_ms: u64,
    pub avg_candidate_latency_ms: u64,
    pub outcome: OptimizationClaimOutcome,
}

/// Evaluator that strictly enforces the G12 invariant:
/// "Không claim cheaper/quality-preserved nếu chưa đạt margin".
pub struct OptimizationEvaluator;

impl OptimizationEvaluator {
    pub fn evaluate(
        baseline_runs: &[TaskExecutionMetrics],
        candidate_runs: &[TaskExecutionMetrics],
        config: &ComparisonMarginConfig,
    ) -> OptimizationEvaluationReport {
        let sample_count = baseline_runs.len().min(candidate_runs.len());

        if sample_count < config.min_sample_size {
            return OptimizationEvaluationReport {
                baseline_harness: baseline_runs
                    .first()
                    .map(|r| r.harness_id.clone())
                    .unwrap_or_default(),
                candidate_harness: candidate_runs
                    .first()
                    .map(|r| r.harness_id.clone())
                    .unwrap_or_default(),
                sample_count,
                avg_baseline_quality: 0.0,
                avg_candidate_quality: 0.0,
                avg_baseline_total_cost: 0.0,
                avg_candidate_total_cost: 0.0,
                avg_baseline_latency_ms: 0,
                avg_candidate_latency_ms: 0,
                outcome: OptimizationClaimOutcome::Inconclusive {
                    samples_observed: sample_count,
                    samples_required: config.min_sample_size,
                },
            };
        }

        let mut sum_base_q = 0.0;
        let mut sum_cand_q = 0.0;
        let mut sum_base_cost = 0.0;
        let mut sum_cand_cost = 0.0;
        let mut sum_base_lat = 0;
        let mut sum_cand_lat = 0;

        for i in 0..sample_count {
            let base = &baseline_runs[i];
            let cand = &candidate_runs[i];

            sum_base_q += base.quality_score;
            sum_cand_q += cand.quality_score;
            sum_base_cost += base.total_economic_cost(config.human_hourly_rate_usd);
            sum_cand_cost += cand.total_economic_cost(config.human_hourly_rate_usd);
            sum_base_lat += base.latency_ms;
            sum_cand_lat += cand.latency_ms;
        }

        let n = sample_count as f64;
        let avg_base_q = sum_base_q / n;
        let avg_cand_q = sum_cand_q / n;
        let avg_base_cost = sum_base_cost / n;
        let avg_cand_cost = sum_cand_cost / n;
        let avg_base_lat = sum_base_lat / sample_count as u64;
        let avg_cand_lat = sum_cand_lat / sample_count as u64;

        let quality_delta = avg_cand_q - avg_base_q;

        // 1. Check Quality Non-Inferiority: Candidate quality cannot drop by more than tolerance margin
        if quality_delta < -config.quality_tolerance_margin {
            return OptimizationEvaluationReport {
                baseline_harness: baseline_runs[0].harness_id.clone(),
                candidate_harness: candidate_runs[0].harness_id.clone(),
                sample_count,
                avg_baseline_quality: avg_base_q,
                avg_candidate_quality: avg_cand_q,
                avg_baseline_total_cost: avg_base_cost,
                avg_candidate_total_cost: avg_cand_cost,
                avg_baseline_latency_ms: avg_base_lat,
                avg_candidate_latency_ms: avg_cand_lat,
                outcome: OptimizationClaimOutcome::QualityInferior {
                    baseline_quality: avg_base_q,
                    candidate_quality: avg_cand_q,
                    quality_delta,
                },
            };
        }

        // 2. Check Cost Reduction Margin: Total cost savings must meet or exceed cost_reduction_margin
        let cost_savings_ratio = if avg_base_cost > 0.0 {
            (avg_base_cost - avg_cand_cost) / avg_base_cost
        } else {
            0.0
        };

        let outcome = if cost_savings_ratio >= config.cost_reduction_margin {
            OptimizationClaimOutcome::QualityPreservedAndCheaper {
                quality_delta,
                cost_savings_ratio,
            }
        } else {
            OptimizationClaimOutcome::NotCheaper {
                baseline_cost: avg_base_cost,
                candidate_cost: avg_cand_cost,
                cost_savings_ratio,
            }
        };

        OptimizationEvaluationReport {
            baseline_harness: baseline_runs[0].harness_id.clone(),
            candidate_harness: candidate_runs[0].harness_id.clone(),
            sample_count,
            avg_baseline_quality: avg_base_q,
            avg_candidate_quality: avg_cand_q,
            avg_baseline_total_cost: avg_base_cost,
            avg_candidate_total_cost: avg_cand_cost,
            avg_baseline_latency_ms: avg_base_lat,
            avg_candidate_latency_ms: avg_cand_lat,
            outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_run(
        task: &str,
        harness: &str,
        quality: f64,
        cost: f64,
        human_sec: u64,
    ) -> TaskExecutionMetrics {
        TaskExecutionMetrics {
            task_id: task.into(),
            harness_id: harness.into(),
            success: quality >= 0.8,
            quality_score: quality,
            tokens_used: 1000,
            cost_usd: cost,
            latency_ms: 1000,
            human_intervention_seconds: human_sec,
            attempts_count: 1,
            recovery_cost_usd: 0.0,
        }
    }

    #[test]
    fn test_inconclusive_when_sample_size_below_threshold() {
        let base = vec![make_run("t1", "baseline", 1.0, 0.10, 0)];
        let cand = vec![make_run("t1", "cheap-agent", 1.0, 0.02, 0)];
        let cfg = ComparisonMarginConfig::default();

        let report = OptimizationEvaluator::evaluate(&base, &cand, &cfg);
        assert!(matches!(
            report.outcome,
            OptimizationClaimOutcome::Inconclusive { .. }
        ));
    }

    #[test]
    fn test_rejects_inferior_quality_even_if_cheap() {
        let base = vec![
            make_run("t1", "baseline", 1.0, 0.10, 0),
            make_run("t2", "baseline", 1.0, 0.10, 0),
            make_run("t3", "baseline", 1.0, 0.10, 0),
        ];
        let cand = vec![
            make_run("t1", "cheap-agent", 0.7, 0.01, 0),
            make_run("t2", "cheap-agent", 0.6, 0.01, 0),
            make_run("t3", "cheap-agent", 0.7, 0.01, 0),
        ];
        let cfg = ComparisonMarginConfig::default();

        let report = OptimizationEvaluator::evaluate(&base, &cand, &cfg);
        assert!(matches!(
            report.outcome,
            OptimizationClaimOutcome::QualityInferior { .. }
        ));
        assert!(!report.outcome.is_admissible_optimization());
    }

    #[test]
    fn test_rejects_apparent_token_savings_when_human_recovery_cost_is_included() {
        let base = vec![
            make_run("t1", "baseline", 1.0, 0.10, 0),
            make_run("t2", "baseline", 1.0, 0.10, 0),
            make_run("t3", "baseline", 1.0, 0.10, 0),
        ];
        // Candidate is $0.02 in tokens, but required 600s (10 min) human fixing per task!
        // At $60/hr, 600s = $10.00 human cost!
        let cand = vec![
            make_run("t1", "cheap-agent", 1.0, 0.02, 600),
            make_run("t2", "cheap-agent", 1.0, 0.02, 600),
            make_run("t3", "cheap-agent", 1.0, 0.02, 600),
        ];
        let cfg = ComparisonMarginConfig::default();

        let report = OptimizationEvaluator::evaluate(&base, &cand, &cfg);
        assert!(matches!(
            report.outcome,
            OptimizationClaimOutcome::NotCheaper { .. }
        ));
        assert!(!report.outcome.is_admissible_optimization());
    }

    #[test]
    fn test_approves_genuine_optimization() {
        let base = vec![
            make_run("t1", "baseline", 1.0, 0.10, 0),
            make_run("t2", "baseline", 1.0, 0.10, 0),
            make_run("t3", "baseline", 1.0, 0.10, 0),
        ];
        let cand = vec![
            make_run("t1", "optimized", 1.0, 0.05, 0),
            make_run("t2", "optimized", 0.98, 0.05, 0),
            make_run("t3", "optimized", 1.0, 0.05, 0),
        ];
        let cfg = ComparisonMarginConfig::default();

        let report = OptimizationEvaluator::evaluate(&base, &cand, &cfg);
        assert!(matches!(
            report.outcome,
            OptimizationClaimOutcome::QualityPreservedAndCheaper { .. }
        ));
        assert!(report.outcome.is_admissible_optimization());
    }
}
