//! Verifier Implementations & Core Trait
//!
//! Provides deterministic verification of evidence bundles into signed/structured VerificationClaims.

use crate::bundle::EvidenceBundle;
use async_trait::async_trait;
use custos_core_domain::{DomainError, VerificationClaim};

#[async_trait]
pub trait Verifier: Send + Sync {
    /// Identifier of this verifier (e.g. "exit_code", "hash", "exact_match").
    fn verifier_id(&self) -> &str;

    /// Verifies the bundle and returns a canonical VerificationClaim.
    async fn verify(&self, bundle: &EvidenceBundle) -> Result<VerificationClaim, DomainError>;
}

/// Verifies command execution success by checking exit code == 0.
pub struct CommandExitCodeVerifier;

#[async_trait]
impl Verifier for CommandExitCodeVerifier {
    fn verifier_id(&self) -> &str {
        "command_exit_code"
    }

    async fn verify(&self, bundle: &EvidenceBundle) -> Result<VerificationClaim, DomainError> {
        let exit_code = bundle
            .proof
            .get("exit_code")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                DomainError::Validation("Missing 'exit_code' integer field in proof".into())
            })?;

        let passed = exit_code == 0;
        let details = serde_json::json!({
            "exit_code": exit_code,
            "stdout": bundle.proof.get("stdout").cloned().unwrap_or(serde_json::Value::Null),
            "stderr": bundle.proof.get("stderr").cloned().unwrap_or(serde_json::Value::Null),
        });

        Ok(VerificationClaim::new(
            bundle.task_id.clone(),
            bundle.claim_statement.clone(),
            self.verifier_id().to_string(),
            passed,
            details,
        ))
    }
}

/// Verifies that an actual digest matches the expected digest.
pub struct HashVerifier;

#[async_trait]
impl Verifier for HashVerifier {
    fn verifier_id(&self) -> &str {
        "hash"
    }

    async fn verify(&self, bundle: &EvidenceBundle) -> Result<VerificationClaim, DomainError> {
        let actual = bundle
            .proof
            .get("actual_digest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                DomainError::Validation("Missing 'actual_digest' string field in proof".into())
            })?;

        let expected = bundle
            .proof
            .get("expected_digest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                DomainError::Validation("Missing 'expected_digest' string field in proof".into())
            })?;

        let passed = actual == expected;
        let details = serde_json::json!({
            "actual_digest": actual,
            "expected_digest": expected,
            "match": passed,
        });

        Ok(VerificationClaim::new(
            bundle.task_id.clone(),
            bundle.claim_statement.clone(),
            self.verifier_id().to_string(),
            passed,
            details,
        ))
    }
}

/// Verifies that output contains or strictly equals an expected string.
pub struct ExactMatchVerifier;

#[async_trait]
impl Verifier for ExactMatchVerifier {
    fn verifier_id(&self) -> &str {
        "exact_match"
    }

    async fn verify(&self, bundle: &EvidenceBundle) -> Result<VerificationClaim, DomainError> {
        let actual = bundle
            .proof
            .get("actual")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                DomainError::Validation("Missing 'actual' string field in proof".into())
            })?;

        let expected = bundle
            .proof
            .get("expected")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                DomainError::Validation("Missing 'expected' string field in proof".into())
            })?;

        let passed = actual == expected;
        let details = serde_json::json!({
            "actual": actual,
            "expected": expected,
            "match": passed,
        });

        Ok(VerificationClaim::new(
            bundle.task_id.clone(),
            bundle.claim_statement.clone(),
            self.verifier_id().to_string(),
            passed,
            details,
        ))
    }
}

/// Verifies that cited file paths and line ranges exist on disk or within a workspace.
pub struct CitationVerifier;

#[async_trait]
impl Verifier for CitationVerifier {
    fn verifier_id(&self) -> &str {
        "citation"
    }

    async fn verify(&self, bundle: &EvidenceBundle) -> Result<VerificationClaim, DomainError> {
        let citations = bundle
            .proof
            .get("citations")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                DomainError::Validation("Missing or invalid 'citations' array in proof".into())
            })?;

        let base_path = bundle
            .proof
            .get("base_path")
            .and_then(|v| v.as_str())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let mut results = Vec::new();
        let mut all_passed = true;

        for (idx, cit) in citations.iter().enumerate() {
            let rel_path = cit.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if rel_path.is_empty() {
                all_passed = false;
                results.push(serde_json::json!({
                    "index": idx,
                    "passed": false,
                    "error": "Empty path in citation"
                }));
                continue;
            }

            let full_path = base_path.join(rel_path);
            if !full_path.exists() {
                all_passed = false;
                results.push(serde_json::json!({
                    "index": idx,
                    "path": rel_path,
                    "passed": false,
                    "error": "File does not exist"
                }));
                continue;
            }

            let file_content = match std::fs::read_to_string(&full_path) {
                Ok(c) => c,
                Err(e) => {
                    all_passed = false;
                    results.push(serde_json::json!({
                        "index": idx,
                        "path": rel_path,
                        "passed": false,
                        "error": format!("Could not read file: {e}")
                    }));
                    continue;
                }
            };

            let lines: Vec<&str> = file_content.lines().collect();
            let total_lines = lines.len();

            let start_line = cit.get("start_line").and_then(|v| v.as_u64()).map(|n| n as usize);
            let end_line = cit.get("end_line").and_then(|v| v.as_u64()).map(|n| n as usize);

            let mut range_ok = true;
            if let Some(start) = start_line {
                if start == 0 || (total_lines > 0 && start > total_lines) {
                    range_ok = false;
                }
            }
            if let Some(end) = end_line {
                if end == 0 || (total_lines > 0 && end > total_lines) {
                    range_ok = false;
                }
                if let Some(start) = start_line {
                    if start > end {
                        range_ok = false;
                    }
                }
            }

            if !range_ok {
                all_passed = false;
                results.push(serde_json::json!({
                    "index": idx,
                    "path": rel_path,
                    "total_lines": total_lines,
                    "start_line": start_line,
                    "end_line": end_line,
                    "passed": false,
                    "error": "Line range is out of bounds or invalid"
                }));
                continue;
            }

            // Optional snippet verification
            if let Some(expected_snippet) = cit.get("snippet").and_then(|v| v.as_str()) {
                let check_text = match (start_line, end_line) {
                    (Some(s), Some(e)) if s <= e && e <= total_lines => {
                        lines[s - 1..e].join("\n")
                    }
                    _ => file_content.clone(),
                };
                if !check_text.contains(expected_snippet) {
                    all_passed = false;
                    results.push(serde_json::json!({
                        "index": idx,
                        "path": rel_path,
                        "passed": false,
                        "error": "Expected snippet not found within cited range"
                    }));
                    continue;
                }
            }

            results.push(serde_json::json!({
                "index": idx,
                "path": rel_path,
                "total_lines": total_lines,
                "start_line": start_line,
                "end_line": end_line,
                "snippet_verified": cit.get("snippet").is_some(),
                "passed": true
            }));
        }

        let details = serde_json::json!({
            "total_citations": citations.len(),
            "all_passed": all_passed,
            "citations": results,
        });

        Ok(VerificationClaim::new(
            bundle.task_id.clone(),
            bundle.claim_statement.clone(),
            self.verifier_id().to_string(),
            all_passed,
            details,
        ))
    }
}
