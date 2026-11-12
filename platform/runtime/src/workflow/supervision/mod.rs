//! Runtime Run Supervision Engine (located inside canonical pillar `workflow`).

pub mod coordinator;
mod processor;
pub mod profile_registry;
pub mod redactor;

pub use coordinator::SupervisionCoordinator;
pub use profile_registry::SupervisionProfileRegistry;
pub use redactor::PayloadRedactor;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use custos_core::contracts::supervision::{
        NormalizedTelemetryBatch, SupervisionIngestPort, SupervisionQueryPort, SupervisionTraceSink,
    };
    use custos_domain::supervision::{
        FindingKind, RunStepSpan, StepSpanKind, StepSpanStatus, SupervisionProfile,
        SupervisionRunRecord, SupervisionRunStatus, ThresholdPolicy,
    };
    use custos_domain::UsageMeasurement;
    use serde_json::json;

    #[test]
    fn redactor_masks_secrets_before_persistence() {
        let redactor = PayloadRedactor::default();
        let redacted = redactor.redact_json(json!({
            "api_key": "sk-secret123456",
            "command": "curl -H 'Authorization: Bearer my-jwt-token' https://example.com"
        }));
        assert_eq!(redacted["api_key"], "[REDACTED]");
        let cmd = redacted["command"].as_str().unwrap();
        assert!(!cmd.contains("my-jwt-token"));
    }

    #[tokio::test]
    async fn coordinator_ingests_and_queries_persisted_waterfall() {
        let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
        let coord = Arc::new(SupervisionCoordinator::new(
            Some(store),
            crate::workflow::live::LiveRuns::default(),
        ));
        coord.profiles().register_profile(
            "coding",
            SupervisionProfile::for_workbench(
                "coding",
                "Coding Workbench",
                ThresholdPolicy {
                    loop_repeat_count: 2,
                    ..Default::default()
                },
            ),
        );

        let batch = NormalizedTelemetryBatch {
            runs: vec![SupervisionRunRecord {
                run_id: "run_test_1".into(),
                workbench_id: Some("coding".into()),
                agent: "code_implementer".into(),
                status: SupervisionRunStatus::Running,
                started_at_ms: 1000,
                ended_at_ms: None,
                duration_ms: None,
                tokens: Some(420),
                cost_usd: None,
                usage_measurement: UsageMeasurement::Known(420),
            }],
            spans: vec![
                RunStepSpan {
                    id: "sp1".into(),
                    run_id: "run_test_1".into(),
                    worker_run_id: Some("w1".into()),
                    parent_id: None,
                    kind: StepSpanKind::Tool,
                    name: "grep_search".into(),
                    status: StepSpanStatus::Success,
                    started_at_ms: 1010,
                    ended_at_ms: Some(1020),
                    duration_ms: Some(10),
                    metadata: json!({ "api_key": "secret-val" }),
                },
                RunStepSpan {
                    id: "sp2".into(),
                    run_id: "run_test_1".into(),
                    worker_run_id: Some("w1".into()),
                    parent_id: None,
                    kind: StepSpanKind::Tool,
                    name: "grep_search".into(),
                    status: StepSpanStatus::Success,
                    started_at_ms: 1030,
                    ended_at_ms: Some(1040),
                    duration_ms: Some(10),
                    metadata: json!({}),
                },
                RunStepSpan {
                    id: "sp3".into(),
                    run_id: "run_test_1".into(),
                    worker_run_id: Some("w1".into()),
                    parent_id: None,
                    kind: StepSpanKind::Tool,
                    name: "grep_search".into(),
                    status: StepSpanStatus::Success,
                    started_at_ms: 1050,
                    ended_at_ms: Some(1060),
                    duration_ms: Some(10),
                    metadata: json!({}),
                },
            ],
            logs: vec![],
            errors: vec![],
        };

        coord.ingest_batch(batch).await.unwrap();
        let waterfall = coord.get_run_waterfall("run_test_1").await.unwrap().unwrap();
        assert_eq!(waterfall.spans.len(), 3);
        assert_eq!(waterfall.spans[0].metadata["api_key"], "[REDACTED]");
        assert!(waterfall
            .findings
            .iter()
            .any(|f| f.kind == FindingKind::PossibleLoop));
        assert_eq!(waterfall.applied_profile.workbench_id.as_deref(), Some("coding"));

        // Non-blocking observer sink smoke call outside background task
        coord.notify_run_state("run_test_1", Some("coding"), "code_implementer", false, 1000, Some(2000));
    }
}
