//! S1 Fast Judge (Rule-Based & Calibrated Heuristic Verifier)
//!
//! Evaluates candidate results against declared contract criteria.
//! If confidence is below the calibrated threshold (default 0.75), S1 abstains
//! and produces an escalation signal to S2.

use super::calibration::ConfidenceCalibrator;
use async_trait::async_trait;
use custos_core::contracts::judgment::{JudgmentPort, JudgmentRequest, JudgmentResult};
use custos_domain::DomainError;
use tracing::{debug, warn};

/// S1 Fast Verifier with calibrated confidence estimation.
#[derive(Debug, Clone)]
pub struct S1Judge {
    calibrator: ConfidenceCalibrator,
}

impl Default for S1Judge {
    fn default() -> Self {
        Self::new()
    }
}

impl S1Judge {
    /// Creates a new S1Judge with standard Brier-calibrated abstain threshold.
    pub fn new() -> Self {
        Self {
            calibrator: ConfidenceCalibrator::new(),
        }
    }

    /// Creates an S1Judge with a custom abstain threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            calibrator: ConfidenceCalibrator::with_threshold(threshold),
        }
    }

    /// Evaluates candidate JSON against a single string criterion.
    fn evaluate_criterion(
        &self,
        criterion: &str,
        candidate: &serde_json::Value,
    ) -> Result<(), String> {
        let crit_lower = criterion.to_lowercase();

        // 1. Check for explicit exit code or status requirements
        if crit_lower.contains("exit code 0") || crit_lower.contains("success") {
            if let Some(code) = candidate.get("exit_code").and_then(|v| v.as_i64()) {
                if code != 0 {
                    return Err(format!("exit_code is {} (expected 0)", code));
                }
            }
            if let Some(status) = candidate.get("status").and_then(|v| v.as_str()) {
                if status == "failed" || status == "error" {
                    return Err(format!("status is '{}'", status));
                }
            }
        }

        // 2. Check for prohibited patterns or errors
        if crit_lower.contains("no errors") || crit_lower.contains("clean") {
            if let Some(err) = candidate.get("error").and_then(|v| v.as_str()) {
                if !err.is_empty() {
                    return Err(format!("candidate contains error: {}", err));
                }
            }
            if let Some(stderr) = candidate.get("stderr").and_then(|v| v.as_str()) {
                if !stderr.is_empty() && (stderr.contains("error:") || stderr.contains("fatal:")) {
                    return Err(format!("candidate stderr contains fatal error: {}", stderr));
                }
            }
        }

        // 3. Keyword / substring presence requirement
        if let Some(expected_kw) = crit_lower.strip_prefix("contains:") {
            let kw = expected_kw.trim();
            let candidate_str = candidate.to_string().to_lowercase();
            if !candidate_str.contains(kw) {
                return Err(format!("candidate missing required token '{}'", kw));
            }
        }

        Ok(())
    }

    /// Computes calibrated confidence based on candidate signal clarity.
    fn compute_confidence(&self, candidate: &serde_json::Value, criteria_count: usize) -> f64 {
        if criteria_count == 0 {
            return 1.0;
        }

        // If candidate is null or empty object, confidence is minimal
        if candidate.is_null() || candidate.as_object().map(|m| m.is_empty()).unwrap_or(false) {
            return 0.10;
        }

        let mut conf: f64 = 0.85;

        // If candidate lacks standard structured execution fields, reduce confidence
        let has_status = candidate.get("status").is_some();
        let has_exit = candidate.get("exit_code").is_some();
        let has_output = candidate.get("output").is_some() || candidate.get("stdout").is_some();

        if !has_status && !has_exit {
            conf -= 0.15;
        }
        if !has_output {
            conf -= 0.10;
        }

        // Check if candidate contains ambiguous or partial indicators
        if let Some(s) = candidate.get("status").and_then(|v| v.as_str()) {
            if s == "partial" || s == "indeterminate" || s == "uncertain" {
                conf = 0.50; // Triggers abstain
            }
        }

        conf.clamp(0.0, 1.0)
    }
}

#[async_trait]
impl JudgmentPort for S1Judge {
    async fn evaluate_evidence(
        &self,
        criteria: &[String],
        payload: &str,
    ) -> Result<f32, DomainError> {
        let val: serde_json::Value =
            serde_json::from_str(payload).unwrap_or(serde_json::Value::Null);
        let conf = self.compute_confidence(&val, criteria.len());
        Ok(conf as f32)
    }

    async fn judge(&self, request: JudgmentRequest) -> Result<JudgmentResult, DomainError> {
        debug!(judgment_id = %request.judgment_id, criteria_count = request.criteria.len(), "S1 judging candidate");

        let mut violations = Vec::new();
        for crit in &request.criteria {
            if let Err(violation) = self.evaluate_criterion(crit, &request.candidate) {
                violations.push(format!("{}: {}", crit, violation));
            }
        }

        let total_criteria = request.criteria.len();
        let satisfied = total_criteria.saturating_sub(violations.len());
        let score = if total_criteria == 0 {
            1.0
        } else {
            satisfied as f64 / total_criteria as f64
        };

        let confidence = self.compute_confidence(&request.candidate, total_criteria);

        // Check abstain threshold (< 0.75)
        if self.calibrator.should_abstain(confidence) {
            warn!(
                judgment_id = %request.judgment_id,
                confidence = confidence,
                threshold = self.calibrator.threshold(),
                "S1 confidence below threshold; abstaining to S2"
            );
            return Ok(JudgmentResult {
                judgment_id: request.judgment_id,
                passed: false,
                score,
                confidence,
                rationale: format!(
                    "S1 Abstain: confidence {:.2} < {:.2} (escalating to S2)",
                    confidence,
                    self.calibrator.threshold()
                ),
                violations: if violations.is_empty() {
                    None
                } else {
                    Some(violations)
                },
            });
        }

        let passed = violations.is_empty() && (score >= 1.0 - 1e-6);
        let rationale = if passed {
            "S1 Verified: all criteria satisfied with calibrated confidence".to_string()
        } else {
            format!(
                "S1 Rejected: {} criteria violations detected",
                violations.len()
            )
        };

        Ok(JudgmentResult {
            judgment_id: request.judgment_id,
            passed,
            score,
            confidence,
            rationale,
            violations: if violations.is_empty() {
                None
            } else {
                Some(violations)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::calibration::ABSTAIN_CONFIDENCE_THRESHOLD;
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_s1_judge_passes_clean_result() {
        let judge = S1Judge::new();
        let req = JudgmentRequest {
            judgment_id: "j-001".into(),
            task_id: "t-001".into(),
            criteria: vec!["exit code 0".into(), "contains: success".into()],
            candidate: json!({
                "status": "completed",
                "exit_code": 0,
                "stdout": "task completed with success"
            }),
            context: None,
        };

        let res = judge.judge(req).await.unwrap();
        assert!(res.passed);
        assert!(res.confidence >= ABSTAIN_CONFIDENCE_THRESHOLD);
        assert_eq!(res.score, 1.0);
    }

    #[tokio::test]
    async fn test_s1_judge_abstains_on_uncertain_signal() {
        let judge = S1Judge::new();
        let req = JudgmentRequest {
            judgment_id: "j-002".into(),
            task_id: "t-002".into(),
            criteria: vec!["success".into()],
            candidate: json!({
                "status": "uncertain",
                "output": "partial execution"
            }),
            context: None,
        };

        let res = judge.judge(req).await.unwrap();
        assert!(!res.passed);
        assert!(res.confidence < ABSTAIN_CONFIDENCE_THRESHOLD);
        assert!(res.rationale.contains("S1 Abstain"));
    }

    #[tokio::test]
    async fn test_s1_judge_rejects_on_violations() {
        let judge = S1Judge::new();
        let req = JudgmentRequest {
            judgment_id: "j-003".into(),
            task_id: "t-003".into(),
            criteria: vec!["exit code 0".into()],
            candidate: json!({
                "status": "failed",
                "exit_code": 1,
                "stdout": "compilation failed"
            }),
            context: None,
        };

        let res = judge.judge(req).await.unwrap();
        assert!(!res.passed);
        assert!(res.confidence >= ABSTAIN_CONFIDENCE_THRESHOLD);
        assert!(res.violations.is_some());
    }
}
