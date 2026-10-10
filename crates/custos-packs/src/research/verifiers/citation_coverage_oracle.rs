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

        let total_claims = claims.len();
        let mut grounded_claims = 0;
        let mut ungrounded_claim_ids = Vec::new();

        for (idx, claim) in claims.iter().enumerate() {
            let has_citation = claim
                .get("citation")
                .or_else(|| claim.get("doi"))
                .and_then(Value::as_str)
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            // B5 Invariant: DOI or citation syntax presence alone NEVER defaults verified to true.
            let self_asserted_verified = claim
                .get("verified")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            // Check grounding level: must not be l0_ungrounded
            let is_ungrounded = claim
                .get("level")
                .or_else(|| claim.get("grounding_level"))
                .and_then(Value::as_str)
                .map(|s| s.eq_ignore_ascii_case("l0_ungrounded") || s.eq_ignore_ascii_case("ungrounded"))
                .unwrap_or(false);

            // Check semantic support & locator status
            let is_unsupported = claim
                .get("semantic_status")
                .or_else(|| claim.get("support_status"))
                .and_then(Value::as_str)
                .map(|s| s.eq_ignore_ascii_case("unsupported") || s.eq_ignore_ascii_case("contradicted"))
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
            // 2. Verified flag is explicitly true
            // 3. Not L0 ungrounded
            // 4. Not unsupported, not invalid locator, not stale
            let is_grounded = has_citation
                && self_asserted_verified
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
                ungrounded_claim_ids.push(id);
            }
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
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2", "verified": true },
                { "id": "c2", "doi": "10.1126/science.1234567", "verified": true }
            ]
        }));

        let res = oracle.verify(&ctx).await;
        assert!(res.is_pass());
    }

    #[tokio::test]
    async fn test_citation_coverage_fail_threshold() {
        let oracle = CitationCoverageOracle::new();
        let ctx = VerificationContext::new("task_res_2").with_metadata(json!({
            "claims": [
                { "id": "c1", "doi": "10.1038/s41586-020-2649-2", "verified": true },
                { "id": "c2", "doi": "", "verified": false }
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
}
