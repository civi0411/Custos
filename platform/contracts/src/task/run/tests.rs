use super::claim::*;
use super::finding::*;
use super::launch::*;
use super::step_span::*;
use super::types::*;
use super::usage::*;
use super::watchdog::*;
use crate::UsageMeasurement;
use serde_json::json;

#[test]
fn test_run_lifecycle() {
    let mut run = Run::new("task_001".into(), 1);
    assert_eq!(run.status, RunStatus::Pending);
    assert!(run.transition(RunStatus::Active).is_ok());
    assert!(run.transition(RunStatus::Suspended).is_ok());
    assert!(run.transition(RunStatus::Active).is_ok());
    assert!(run.transition(RunStatus::Completed).is_ok());
    assert!(run.status.is_terminal());
    assert!(run.ended_at.is_some());
    assert!(run.transition(RunStatus::Active).is_err());
}

#[test]
fn test_worker_run_bounded_retry() {
    let mut wrun = WorkerRun::new("task_002".into(), "worker_alpha".into(), 3);
    assert_eq!(wrun.attempt_id, 1);
    assert!(wrun.can_retry());

    assert_eq!(wrun.record_attempt().unwrap(), 2);
    assert!(wrun.can_retry());

    assert_eq!(wrun.record_attempt().unwrap(), 3);
    assert!(!wrun.can_retry());

    // Exceeded bounded retry
    assert!(wrun.record_attempt().is_err());
}

#[test]
fn test_dispatch_claim_lifecycle() {
    let mut claim = DispatchClaim::new("task_claim_1", "worker_1", 1);
    assert_eq!(claim.status, ClaimStatus::Pending);
    assert_eq!(claim.depth, 1);
    assert!(claim.dispatched_at.is_none());

    assert!(claim.mark_dispatched("wrun_1").is_ok());
    assert_eq!(claim.status, ClaimStatus::Dispatched);
    assert_eq!(claim.worker_run_id, Some("wrun_1".to_string()));
    assert!(claim.dispatched_at.is_some());

    // Cannot transition from dispatched to dispatched again
    assert!(claim.mark_dispatched("wrun_2").is_err());

    // Release
    assert!(claim.release().is_ok());
    assert_eq!(claim.status, ClaimStatus::Released);
    assert!(claim.released_at.is_some());
}

#[test]
fn unknown_launch_cannot_be_blindly_retried_or_reclassified() {
    let mut attempt = LaunchAttempt::new("worker_run_1", "native");
    assert!(attempt.is_ok());
    if let Ok(ref mut value) = attempt {
        assert!(value
            .mark_unknown("attach outcome was not observed")
            .is_ok());
        assert!(!value.status.allows_retry());
        assert!(value.mark_launched("native", Some("codex".into())).is_err());
    }
}

#[test]
fn usage_defaults_to_unknown_instead_of_zero() {
    let usage = UsageRecord::new(
        "task_1",
        "run_1",
        "attempt_1",
        UsageExecutorKind::NativeHarness,
        "codex",
    );
    assert!(usage.is_ok());
    if let Ok(value) = usage {
        assert_eq!(value.input_tokens, UsageMeasurement::Unknown);
        assert_eq!(value.cost_micros, UsageMeasurement::Unknown);
    }
}

fn make_tool_span(
    id: &str,
    worker: Option<&str>,
    parent: Option<&str>,
    name: &str,
    started: i64,
) -> RunStepSpan {
    RunStepSpan {
        id: id.into(),
        run_id: "run_1".into(),
        worker_run_id: worker.map(str::to_string),
        parent_id: parent.map(str::to_string),
        kind: StepSpanKind::Tool,
        name: name.into(),
        status: StepSpanStatus::Success,
        started_at_ms: started,
        ended_at_ms: Some(started + 50),
        duration_ms: Some(50),
        metadata: json!({}),
    }
}

#[test]
fn parallel_dag_workers_do_not_trigger_false_loop() {
    let spans = vec![
        make_tool_span("s1", Some("wA"), Some("pA"), "search", 100),
        make_tool_span("s2", Some("wB"), Some("pB"), "search", 105),
        make_tool_span("s3", Some("wC"), Some("pC"), "search", 110),
    ];
    let profile = SupervisionProfile::default();
    let rules = default_rules();
    let snap = SupervisionSnapshot {
        run_id: "run_1",
        agent: "coding-worker",
        is_active: false,
        started_at_ms: 0,
        ended_at_ms: Some(200),
        duration_ms: Some(200),
        spans: &spans,
        logs: &[],
        errors: &[],
        now_ms: 300,
    };
    let findings = evaluate_snapshot(&rules, &snap, &profile);
    assert!(
        !findings.iter().any(|f| f.kind == FindingKind::PossibleLoop),
        "parallel workers must not falsely trigger PossibleLoop"
    );
}

#[test]
fn single_branch_cycle_triggers_possible_loop() {
    let spans = vec![
        make_tool_span("s1", Some("w1"), Some("root"), "grep", 100),
        make_tool_span("s2", Some("w1"), Some("root"), "edit", 200),
        make_tool_span("s3", Some("w1"), Some("root"), "grep", 300),
        make_tool_span("s4", Some("w1"), Some("root"), "edit", 400),
    ];
    let profile = SupervisionProfile::default();
    let rules = default_rules();
    let snap = SupervisionSnapshot {
        run_id: "run_1",
        agent: "coding-worker",
        is_active: true,
        started_at_ms: 0,
        ended_at_ms: None,
        duration_ms: None,
        spans: &spans,
        logs: &[],
        errors: &[],
        now_ms: 500,
    };
    let findings = evaluate_snapshot(&rules, &snap, &profile);
    assert!(
        findings.iter().any(|f| f.kind == FindingKind::PossibleLoop),
        "alternating loop on single branch must trigger PossibleLoop"
    );
}

#[test]
fn error_signature_normalizes_ids_and_hex() {
    let raw_msg_1 = "Failed to query SQLite: record not found with id 550e8400-e29b-41d4-a716-446655440000 at 0xdeadbeef";
    let raw_msg_2 = "Failed to query SQLite: record not found with id 123e4567-e89b-12d3-a456-426614174000 at 0xcafe0000";
    let (norm1, sig1) = compute_error_signature("DatabaseError", raw_msg_1);
    let (norm2, sig2) = compute_error_signature("DatabaseError", raw_msg_2);
    assert_eq!(norm1, norm2);
    assert_eq!(sig1, sig2);
}

#[test]
fn latency_breakdown_correctly_aggregates() {
    let spans = vec![
        RunStepSpan {
            id: "sp1".into(),
            run_id: "r1".into(),
            worker_run_id: None,
            parent_id: None,
            kind: StepSpanKind::Llm,
            name: "call_model".into(),
            status: StepSpanStatus::Success,
            started_at_ms: 0,
            ended_at_ms: Some(1500),
            duration_ms: Some(1500),
            metadata: json!({}),
        },
        RunStepSpan {
            id: "sp2".into(),
            run_id: "r1".into(),
            worker_run_id: None,
            parent_id: None,
            kind: StepSpanKind::Tool,
            name: "cargo_check".into(),
            status: StepSpanStatus::Success,
            started_at_ms: 1500,
            ended_at_ms: Some(2500),
            duration_ms: Some(1000),
            metadata: json!({}),
        },
    ];
    let breakdown = LatencyBreakdown::compute(&spans);
    assert_eq!(breakdown.llm_ms, 1500);
    assert_eq!(breakdown.tool_ms, 1000);
    assert_eq!(breakdown.total_ms, 2500);
}
