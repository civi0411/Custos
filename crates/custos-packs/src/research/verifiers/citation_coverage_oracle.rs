//! Citation Coverage Oracle
//!
//! Preferred verifier for Research Pack (`citation_coverage_oracle`).
//! Assesses whether factual and scientific claims produced in research synthesis
//! meet the mandatory citation grounding threshold before Gate 4 completion.

use async_trait::async_trait;
use serde_json::Value;

use crate::verifier::{PackVerifier, VerificationContext, VerificationResult};

pub struct CitationCoverageOracle {
    default_min_ratio: f64,
}

impl Default for CitationCoverageOracle {
    fn default() -> Self {
        Self {
            default_min_ratio: 0.80, // 80% citation grounding required
        }
    }
}

impl CitationCoverageOracle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threshold(min_ratio: f64) -> Self {
        Self {
            default_min_ratio: min_ratio,
        }
    }
}

#[async_trait]
impl PackVerifier for CitationCoverageOracle {
    fn verifier_id(&self) -> &'static str {
        "citation_coverage_oracle"
    }

    fn description(&self) -> &'static str {
        "Validates that scientific claims have verified DOI/literature backing >= threshold"
    }

    async fn verify(&self, ctx: &VerificationContext) -> VerificationResult {
        let claims = match ctx.metadata.get("claims").and_then(Value::as_array) {
            Some(c) => c,
            None => {
                return VerificationResult::Unknown {
                    reason: "missing 'claims' array in verification context metadata".into(),
                };
            }
        };

        if claims.is_empty() {
            return VerificationResult::Fail {
                reason: "no claims found in research synthesis".into(),
            };
        }

        let min_ratio = ctx
            .metadata
            .get("min_coverage_ratio")
            .and_then(Value::as_f64)
            .unwrap_or(self.default_min_ratio);

        if !(0.0..=1.0).contains(&min_ratio) {
            return VerificationResult::Fail {
                reason: format!(
                    "invalid min_coverage_ratio {}: must be between 0.0 and 1.0",
                    min_ratio
                ),
            };
        }

        let total_claims = claims.len();
        let mut grounded_claims = 0;
        let mut ungrounded_claim_ids = Vec::new();
        let mut unmet_required_claim_ids = Vec::new();

        let assessment_records = ctx
            .metadata
            .get("assessment_records")
            .and_then(Value::as_array);

        for (idx, claim) in claims.iter().enumerate() {
            let claim_id = claim
                .get("id")
                .and_then(Value::as_str)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("claim_{}", idx));

            let is_required = claim
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            let has_citation = claim
                .get("citation")
                .or_else(|| claim.get("doi"))
                .and_then(Value::as_str)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            // B5/C1 Invariant: Only trust external assessment_records with explicit typed status.
            // Self-asserted booleans or missing records are strictly rejected (fail-closed).
            let is_verified_by_assessment = if let Some(records) = assessment_records {
                records.iter().any(|rec| {
                    let matches_id = rec
                        .get("claim_id")
                        .or_else(|| rec.get("target_id"))
                        .or_else(|| rec.get("id"))
                        .and_then(Value::as_str)
                        .map(|id| id == claim_id)
                        .unwrap_or(false);
                    let has_valid_status = rec
                        .get("status")
                        .and_then(Value::as_str)
                        .map(|s| {
                            s.eq_ignore_ascii_case("verified")
                                || s.eq_ignore_ascii_case("approved")
                                || s.eq_ignore_ascii_case("passed")
                        })
                        .unwrap_or(false);
                    let is_stale = rec.get("is_stale").and_then(Value::as_bool).unwrap_or(false)
                        || rec.get("is_fresh").and_then(Value::as_bool).map(|fresh| !fresh).unwrap_or(false);
                    matches_id && has_valid_status && !is_stale
                })
            } else {
                // FAIL-CLOSED: No assessment records means unverified. Self-asserted claim["verified"] is untrusted.
                false
            };

            // Check grounding level: must not be l0_ungrounded
            let is_ungrounded = claim
                .get("level")
                .or_else(|| claim.get("grounding_level"))
                .and_then(Value::as_str)
                .map(|s| {
                    s.eq_ignore_ascii_case("l0_ungrounded") || s.eq_ignore_ascii_case("ungrounded")
                })
                .unwrap_or(false);

            // Check semantic support & locator status
            let is_unsupported = claim
                .get("semantic_status")
                .or_else(|| claim.get("support_status"))
                .or_else(|| claim.get("evidence_support"))
                .and_then(Value::as_str)
                .map(|s| {
                    s.eq_ignore_ascii_case("unsupported")
                        || s.eq_ignore_ascii_case("contradicted")
                        || s.eq_ignore_ascii_case("unknown")
                })
                .unwrap_or(false);

            let is_locator_invalid = claim
                .get("locator_status")
                .and_then(Value::as_str)
                .map(|s| s.eq_ignore_ascii_case("invalid") || s.eq_ignore_ascii_case("missing"))
                .unwrap_or(false);

            let is_stale = claim
                .get("is_stale")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            // Grounded & verified requires:
            // 1. Valid citation/DOI
            // 2. Verified flag backed by non-stale typed assessment
            // 3. Not L0 ungrounded
            // 4. Not unsupported, not invalid locator, not stale
            let is_grounded = has_citation
                && is_verified_by_assessment
                && !is_ungrounded
                && !is_unsupported
                && !is_locator_invalid
                && !is_stale;

            if is_grounded {
                grounded_claims += 1;
            } else {
                let id = claim
                    .get("id")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("claim_{}", idx));
                if is_required {
                    unmet_required_claim_ids.push(id.clone());
                }
                ungrounded_claim_ids.push(id);
            }
        }

        // Required claims are independent of aggregate ratio: all required claims must be grounded.
        if !unmet_required_claim_ids.is_empty() {
            return VerificationResult::Fail {
                reason: format!(
                    "Mandatory required claims were not grounded: {}",
                    unmet_required_claim_ids.join(", ")
                ),
            };
        }

        let ratio = grounded_claims as f64 / total_claims as f64;

        if ratio >= min_ratio {
            VerificationResult::Pass {
                detail: format!(
                    "Citation coverage {:.1}% ({}/{}) meets threshold {:.1}%",
                    ratio * 100.0,
                    grounded_claims,
                    total_claims,
                    min_ratio * 100.0
                ),
            }
        } else {
            VerificationResult::Fail {
                reason: format!(
                    "Citation coverage {:.1}% ({}/{}) below required threshold {:.1}%. Missing citations for: {}",
                    ratio * 100.0,
                    grounded_claims,
                    total_claims,
                    min_ratio * 100.0,
                    ungrounded_claim_ids.join(", ")
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_citation_coverage_pass() {
        let oracle = CitationCoverageOracle::new();
        let ctx = VerificationContext::new("task_res_1").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2" },
                { "id": "c2", "doi": "10.1126/science.1234567" }
            ],
            "assessment_records": [
                { "claim_id": "c1", "status": "verified", "is_stale": false },
                { "claim_id": "c2", "status": "approved", "is_stale": false }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(res.is_pass());
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_untyped_passed_boolean() {
        let oracle = CitationCoverageOracle::new();
        // Assessment records provide untyped boolean without typed status -> rejected!
        let ctx = VerificationContext::new("task_untyped").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2" }
            ],
            "assessment_records": [
                { "claim_id": "c1", "passed": true, "is_stale": false }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_self_asserted_verified_flag_without_assessment() {
        let oracle = CitationCoverageOracle::new();
        // Claim falsely asserts verified: true without assessment_records -> MUST FAIL (fail-closed)
        let ctx = VerificationContext::new("task_res_forged").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2", "verified": true },
                { "id": "c2", "doi": "10.1126/science.1234567", "verified": true }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_fail_threshold() {
        let oracle = CitationCoverageOracle::new();
        let ctx = VerificationContext::new("task_res_2").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2" },
                { "id": "c2", "doi": "" }
            ],
            "assessment_records": [
                { "claim_id": "c1", "status": "verified", "is_stale": false }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_unverified_even_with_doi() {
        let oracle = CitationCoverageOracle::new();
        // Claims have a DOI but verified is missing (previously falsely passed!)
        let ctx = VerificationContext::new("task_res_3").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2" },
                { "id": "c2", "doi": "10.1126/science.1234567" }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        // Must FAIL because presence of DOI alone never grants verified!
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_self_asserted_l0_ungrounded() {
        let oracle = CitationCoverageOracle::new();
        // Caller claims verified: true, but grounding level is l0_ungrounded or semantic_status is unsupported
        let ctx = VerificationContext::new("task_res_4").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2", "verified": true, "level": "l0_ungrounded" },
                { "id": "c2", "doi": "10.1126/science.1234567", "verified": true, "semantic_status": "unsupported" }
            ]
        }));
        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_unknown_support_status() {
        let oracle = CitationCoverageOracle::new();
        // Claims have valid DOI, but support_status is unknown -> must FAIL
        let ctx = VerificationContext::new("task_res_5").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2", "verified": true, "support_status": "unknown" }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_with_trusted_assessment_records_pass() {
        let oracle = CitationCoverageOracle::new();
        let ctx = VerificationContext::new("task_res_6").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2" }
            ],
            "assessment_records": [
                { "claim_id": "c1", "status": "verified", "is_stale": false }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(res.is_pass());
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_forged_metadata_when_assessment_records_missing_or_failed() {
        let oracle = CitationCoverageOracle::new();
        // Claim falsely asserts verified: true, but assessment_records says rejected
        let ctx = VerificationContext::new("task_res_7").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2", "verified": true }
            ],
            "assessment_records": [
                { "claim_id": "c1", "status": "rejected", "is_stale": false }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_unmet_required_claim_even_if_ratio_high() {
        let oracle = CitationCoverageOracle::new();
        // 4 claims total: 3 pass, 1 required fails -> ratio = 75% which exceeds min_ratio 50%, but required fails!
        let ctx = VerificationContext::new("task_required").with_metadata(json!({
            "min_coverage_ratio": 0.50,
            "claims": [
                { "id": "c1", "doi": "10.1038/1", "required": false },
                { "id": "c2", "doi": "10.1038/2", "required": false },
                { "id": "c3", "doi": "10.1038/3", "required": false },
                { "id": "c4", "doi": "10.1038/4", "required": true }
            ],
            "assessment_records": [
                { "claim_id": "c1", "status": "verified", "is_stale": false },
                { "claim_id": "c2", "status": "verified", "is_stale": false },
                { "claim_id": "c3", "status": "verified", "is_stale": false },
                { "claim_id": "c4", "status": "pending", "is_stale": false }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_rejects_invalid_min_ratio() {
        let oracle = CitationCoverageOracle::new();
        let ctx = VerificationContext::new("task_ratio").with_metadata(json!({
            "min_coverage_ratio": 1.5,
            "claims": [
                { "id": "c1", "doi": "10.1038/1" }
            ],
            "assessment_records": [
                { "claim_id": "c1", "status": "verified", "is_stale": false }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(matches!(res, VerificationResult::Fail { .. }));
    }

    #[tokio::test]
    async fn test_citation_coverage_approved_status_with_target_id_passes() {
        let oracle = CitationCoverageOracle::new();
        let ctx = VerificationContext::new("task_target").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2" }
            ],
            "assessment_records": [
                { "target_id": "c1", "status": "approved", "is_fresh": true }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(res.is_pass());
    }
}
