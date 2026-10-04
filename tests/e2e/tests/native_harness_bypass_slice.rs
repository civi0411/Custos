//! Native Harness Adapter & Native Bypass Verification (PR 3 - Gate 4)
//!
//! Reorganized into clear thematic test modules:
//! 1. `harness_profile_and_gate4_invariants`: Honest profile publication and strict rejection of fraudulent CustosMediated claims.
//! 2. `native_bypass_effect_tracking`: Parsing and tracking native shell/file bypasses with typed ProviderGoverned assurance.
//! 3. `harness_lifecycle_steer_and_cancel`: Steering guidance injection and cancellation semantics.

use custos_adapters::harness::ClaudeCodeHarnessAdapter;
use custos_core::contracts::harness::{AgentRuntimePort, ToolMediationLevel};
use custos_domain::{new_id, ActionIntent, Assurance, ContextPack, RiskLevel, WorkerRun};

// =========================================================================
// THEME 1: Honest Profile Publication & Gate 4 Fraudulent Claim Rejection
// =========================================================================
mod harness_profile_and_gate4_invariants {
    use super::*;

    #[tokio::test]
    async fn test_honest_profile_and_fraudulent_claim_rejection() {
        let temp_ws = std::env::temp_dir().join(format!("custos_harness_prof_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_ws).unwrap();
        let adapter = ClaudeCodeHarnessAdapter::new(&temp_ws).with_mock_mode();
        let profile = adapter.profile();

        assert_eq!(profile.harness_id, "claude-code");
        assert_eq!(
            profile.tool_mediation,
            ToolMediationLevel::ProviderGoverned,
            "Claude Code CLI must declare provider-governed mediation, never custos-mediated"
        );
        assert!(!profile.is_custos_governed());
        assert!(profile.supports_cancel);
        assert!(profile.supports_steer);

        // Gate 4 Invariant: Fraudulent attempt to claim 'custos-mediated' MUST be rejected
        let fraudulent_intent = ActionIntent::new(
            new_id("act_fake"),
            "shell_exec".into(),
            "native_shell".into(),
            serde_json::json!({ "command": "cargo run" }),
            RiskLevel::High,
        )
        .with_assurance(Assurance::CustosMediated);

        let violation_err = profile
            .validate_intent_assurance(&fraudulent_intent)
            .expect_err("Fraudulent claim of custos-mediated by unmediated harness must be rejected");

        assert!(
            violation_err.to_string().contains("Gate 4 Violation"),
            "Error message must indicate Gate 4 Violation: {}",
            violation_err
        );

        let _ = std::fs::remove_dir_all(&temp_ws);
    }
}

// =========================================================================
// THEME 2: Native Bypass Effect Tracking
// =========================================================================
mod native_bypass_effect_tracking {
    use super::*;

    #[tokio::test]
    async fn test_observed_intents_labeled_provider_governed() {
        let temp_ws = std::env::temp_dir().join(format!("custos_harness_track_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_ws).unwrap();
        let adapter = ClaudeCodeHarnessAdapter::new(&temp_ws).with_mock_mode();
        let profile = adapter.profile();

        let simulated_cli_output = r#"
[Claude Code Session]
$ git status
$ cargo test --lib
Writing file: src/solution.rs
Editing file: src/domain.rs
Reading file: docs/plan.md
Process finished with code 0.
"#;
        adapter.push_mock_response(simulated_cli_output).await;

        let mut worker_run = WorkerRun::new("task_native_001".into(), "claude-code".into(), 1);
        worker_run.run_id = Some("run_gate4_001".into());

        let context_pack = ContextPack::new("pack_001".into(), vec![], 0, "sha256:test_digest".into());
        let observed_intents = adapter
            .execute_turn(&worker_run, &context_pack)
            .await
            .expect("turn execution succeeds");

        assert_eq!(observed_intents.len(), 5);

        for intent in &observed_intents {
            assert_eq!(intent.assurance, Assurance::ProviderGoverned);
            assert_eq!(intent.parameters["assurance"], "provider-governed");
            assert_eq!(intent.parameters["provider_governed"], true);
            assert_eq!(intent.task_id.as_deref(), Some("task_native_001"));
            profile.validate_intent_assurance(intent).expect("must pass validation");
        }

        assert_eq!(observed_intents[0].name, "shell_exec");
        assert_eq!(observed_intents[1].parameters["command"], "cargo test --lib");
        assert_eq!(observed_intents[2].target, "src/solution.rs");
        assert_eq!(observed_intents[3].target, "src/domain.rs");
        assert_eq!(observed_intents[4].target, "docs/plan.md");

        let _ = std::fs::remove_dir_all(&temp_ws);
    }
}

// =========================================================================
// THEME 3: Harness Lifecycle Steering & Cancellation
// =========================================================================
mod harness_lifecycle_steer_and_cancel {
    use super::*;

    #[tokio::test]
    async fn test_lifecycle_steering_and_cancellation() {
        let temp_ws = std::env::temp_dir().join(format!("custos_harness_life_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_ws).unwrap();
        let adapter = ClaudeCodeHarnessAdapter::new(&temp_ws).with_mock_mode();

        adapter
            .steer_run("run_gate4_002", "Focus on domain errors and avoid git mutations")
            .await
            .expect("steering succeeds");

        adapter.cancel_run("run_gate4_002").await.expect("cancel_run succeeds cleanly");

        let _ = std::fs::remove_dir_all(&temp_ws);
    }
}
