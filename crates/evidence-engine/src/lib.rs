//! Evidence Engine
//!
//! Deterministic verification pipeline, evidence bundles, and requirement validation.

pub mod bundle;
pub mod pipeline;
pub mod verifier;

pub use bundle::*;
pub use pipeline::*;
pub use verifier::*;

#[cfg(test)]
mod tests {
    use super::*;
    use custos_core_domain::EvidenceRequirement;

    #[tokio::test]
    async fn test_command_exit_code_verifier_success() {
        let pipeline = EvidencePipeline::with_standard_verifiers();
        let bundle = EvidenceBundle::new(
            "task_1",
            "command_exit_code",
            "Build succeeded",
            serde_json::json!({
                "exit_code": 0,
                "stdout": "Build OK",
            }),
        );

        let claim = pipeline.verify_bundle(&bundle).await.unwrap();
        assert!(claim.passed);
        assert_eq!(claim.verifier_id, "command_exit_code");
    }

    #[tokio::test]
    async fn test_command_exit_code_verifier_failure() {
        let pipeline = EvidencePipeline::with_standard_verifiers();
        let bundle = EvidenceBundle::new(
            "task_1",
            "command_exit_code",
            "Build succeeded",
            serde_json::json!({
                "exit_code": 1,
                "stderr": "Compilation error",
            }),
        );

        let claim = pipeline.verify_bundle(&bundle).await.unwrap();
        assert!(!claim.passed);
    }

    #[tokio::test]
    async fn test_hash_verifier() {
        let pipeline = EvidencePipeline::with_standard_verifiers();
        let bundle = EvidenceBundle::new(
            "task_2",
            "hash",
            "Checksum matches",
            serde_json::json!({
                "actual_digest": "sha256:abc12345",
                "expected_digest": "sha256:abc12345",
            }),
        );

        let claim = pipeline.verify_bundle(&bundle).await.unwrap();
        assert!(claim.passed);
    }

    #[tokio::test]
    async fn test_pipeline_evaluate_requirements() {
        let pipeline = EvidencePipeline::with_standard_verifiers();

        let reqs = vec![
            EvidenceRequirement {
                kind: "command_exit_code".to_string(),
                description: "Tests pass".to_string(),
                required: true,
                satisfied: false,
                evidence_ref: None,
            },
            EvidenceRequirement {
                kind: "hash".to_string(),
                description: "Artifact hash verified".to_string(),
                required: true,
                satisfied: false,
                evidence_ref: None,
            },
        ];

        let bundle1 = EvidenceBundle::new(
            "task_3",
            "command_exit_code",
            "Tests pass",
            serde_json::json!({"exit_code": 0}),
        );
        let claim1 = pipeline.verify_bundle(&bundle1).await.unwrap();

        // With only claim1, evaluation should be incomplete
        let eval1 = pipeline.evaluate_requirements(&reqs, std::slice::from_ref(&claim1));
        assert!(!eval1.all_satisfied);
        assert_eq!(eval1.satisfied_count, 1);

        // Add passing claim 2
        let bundle2 = EvidenceBundle::new(
            "task_3",
            "hash",
            "Artifact hash verified",
            serde_json::json!({
                "actual_digest": "sha256:xyz",
                "expected_digest": "sha256:xyz",
            }),
        );
        let claim2 = pipeline.verify_bundle(&bundle2).await.unwrap();

        let eval2 = pipeline.evaluate_requirements(&reqs, &[claim1, claim2]);
        assert!(eval2.all_satisfied);
        assert_eq!(eval2.satisfied_count, 2);
        assert!(eval2.unsatisfied_requirements.is_empty());
    }

    #[tokio::test]
    async fn test_citation_verifier_valid_and_invalid() {
        let pipeline = EvidencePipeline::with_standard_verifiers();
        let temp_dir = std::env::temp_dir().join(format!("custos_cite_test_{}", custos_core_domain::new_id("test")));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("sample.rs");
        std::fs::write(&file_path, "line 1: fn main() {\nline 2:     println!(\"hello\");\nline 3: }\n").unwrap();

        // Valid citation with existing file, valid range and matching snippet
        let valid_bundle = EvidenceBundle::new(
            "task_cite_1",
            "citation",
            "Cited sample file exists and matches",
            serde_json::json!({
                "base_path": temp_dir.to_str().unwrap(),
                "citations": [
                    {
                        "path": "sample.rs",
                        "start_line": 1,
                        "end_line": 3,
                        "snippet": "println!(\"hello\")"
                    }
                ]
            }),
        );
        let claim_valid = pipeline.verify_bundle(&valid_bundle).await.unwrap();
        assert!(claim_valid.passed);
        assert_eq!(claim_valid.verifier_id, "citation");

        // Invalid citation (non-existent file)
        let invalid_bundle = EvidenceBundle::new(
            "task_cite_2",
            "citation",
            "Cited non-existent file",
            serde_json::json!({
                "base_path": temp_dir.to_str().unwrap(),
                "citations": [
                    {
                        "path": "non_existent.rs",
                        "start_line": 1,
                        "end_line": 2
                    }
                ]
            }),
        );
        let claim_invalid = pipeline.verify_bundle(&invalid_bundle).await.unwrap();
        assert!(!claim_invalid.passed);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
