//! Deterministic watchdog rules, per-workbench supervision profiles, and run inspection projections.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

use super::finding::{FindingKind, SupervisionFinding, SupervisionSeverity};
use super::step_span::{
    LatencyBreakdown, RunStepSpan, StepError, StepSpanKind, StepSpanStatus, SupervisionLogEntry,
};
use crate::UsageMeasurement;

/// Threshold configuration for deterministic watchdog rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThresholdPolicy {
    pub long_running_ms: i64,
    pub repeated_tool_count: usize,
    pub repeated_error_count: usize,
    pub no_activity_ms: i64,
    pub possible_loop_window: usize,
    #[serde(default)]
    pub loop_repeat_count: usize,
}

impl Default for ThresholdPolicy {
    fn default() -> Self {
        Self {
            long_running_ms: 120_000,
            repeated_tool_count: 5,
            repeated_error_count: 3,
            no_activity_ms: 30_000,
            possible_loop_window: 12,
            loop_repeat_count: 3,
        }
    }
}

/// Workbench-specific execution supervision profile (Strategy Pattern).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupervisionProfile {
    pub profile_id: String,
    pub workbench_id: Option<String>,
    pub thresholds: ThresholdPolicy,
    pub exempt_span_kinds: Vec<StepSpanKind>,
    pub secret_attribute_keys: Vec<String>,
    pub redaction_enabled: bool,
    pub max_payload_bytes: usize,
}

impl Default for SupervisionProfile {
    fn default() -> Self {
        Self {
            profile_id: "default".into(),
            workbench_id: None,
            thresholds: ThresholdPolicy::default(),
            exempt_span_kinds: vec![StepSpanKind::Session, StepSpanKind::Verification],
            secret_attribute_keys: vec![
                "api_key".into(),
                "token".into(),
                "secret".into(),
                "password".into(),
                "authorization".into(),
                "cookie".into(),
            ],
            redaction_enabled: true,
            max_payload_bytes: 65_536,
        }
    }
}

impl SupervisionProfile {
    pub fn for_workbench(
        workbench_id: impl Into<String>,
        _description: impl Into<String>,
        mut thresholds: ThresholdPolicy,
    ) -> Self {
        let wb = workbench_id.into();
        if thresholds.loop_repeat_count > 0 && thresholds.possible_loop_window == 0 {
            thresholds.possible_loop_window = thresholds.loop_repeat_count * 4;
        }
        Self {
            profile_id: wb.clone(),
            workbench_id: Some(wb),
            thresholds,
            exempt_span_kinds: vec![StepSpanKind::Session, StepSpanKind::Verification],
            secret_attribute_keys: vec![
                "api_key".into(),
                "token".into(),
                "secret".into(),
                "password".into(),
                "authorization".into(),
                "cookie".into(),
            ],
            redaction_enabled: true,
            max_payload_bytes: 65_536,
        }
    }

    pub fn with_exempt_kinds(mut self, kinds: Vec<StepSpanKind>) -> Self {
        self.exempt_span_kinds = kinds;
        self
    }

    pub fn is_span_kind_exempt(&self, kind: StepSpanKind) -> bool {
        self.exempt_span_kinds.contains(&kind)
    }
}

/// Point-in-time read-only snapshot of a run evaluated by deterministic watchdog rules.
#[derive(Debug, Clone)]
pub struct SupervisionSnapshot<'a> {
    pub run_id: &'a str,
    pub agent: &'a str,
    pub is_active: bool,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub spans: &'a [RunStepSpan],
    pub logs: &'a [SupervisionLogEntry],
    pub errors: &'a [StepError],
    pub now_ms: i64,
}

/// Strategy contract implemented by every deterministic watchdog rule.
pub trait SupervisionRule: Send + Sync {
    fn rule_id(&self) -> &'static str;
    fn evaluate(
        &self,
        snapshot: &SupervisionSnapshot<'_>,
        profile: &SupervisionProfile,
    ) -> Vec<SupervisionFinding>;
}

/// Evaluates all registered rules deterministically over a snapshot and policy.
pub fn evaluate_snapshot(
    rules: &[Box<dyn SupervisionRule>],
    snapshot: &SupervisionSnapshot<'_>,
    profile: &SupervisionProfile,
) -> Vec<SupervisionFinding> {
    let mut findings = Vec::new();
    for rule in rules {
        findings.extend(rule.evaluate(snapshot, profile));
    }
    findings
}

/// Canonical default deterministic watchdog rule suite.
pub fn default_rules() -> Vec<Box<dyn SupervisionRule>> {
    vec![
        Box::new(LongRunningRule),
        Box::new(RepeatedToolRule),
        Box::new(RepeatedErrorRule),
        Box::new(NoActivityRule),
        Box::new(PossibleLoopRule),
    ]
}

fn format_duration(ms: i64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", (ms as f64) / 1000.0)
    }
}

fn has_active_exempt_span(snapshot: &SupervisionSnapshot<'_>, profile: &SupervisionProfile) -> bool {
    snapshot.spans.iter().any(|span| {
        span.status == StepSpanStatus::Running && profile.is_span_kind_exempt(span.kind)
    })
}

pub struct LongRunningRule;

impl SupervisionRule for LongRunningRule {
    fn rule_id(&self) -> &'static str {
        "long_running"
    }

    fn evaluate(
        &self,
        snapshot: &SupervisionSnapshot<'_>,
        profile: &SupervisionProfile,
    ) -> Vec<SupervisionFinding> {
        if has_active_exempt_span(snapshot, profile) {
            return Vec::new();
        }
        let duration = snapshot.duration_ms.or_else(|| {
            if snapshot.is_active {
                Some(snapshot.now_ms.saturating_sub(snapshot.started_at_ms))
            } else {
                None
            }
        });
        let Some(duration) = duration else {
            return Vec::new();
        };
        let threshold = profile.thresholds.long_running_ms;
        if threshold <= 0 || duration <= threshold {
            return Vec::new();
        }
        vec![SupervisionFinding {
            id: format!("find_lr_{}_{}", snapshot.run_id, snapshot.now_ms),
            run_id: snapshot.run_id.into(),
            kind: FindingKind::LongRunning,
            severity: SupervisionSeverity::Warning,
            message: format!(
                "Run duration ({}) exceeded threshold ({})",
                format_duration(duration),
                format_duration(threshold)
            ),
            created_at_ms: snapshot.now_ms,
        }]
    }
}

pub struct RepeatedToolRule;

impl SupervisionRule for RepeatedToolRule {
    fn rule_id(&self) -> &'static str {
        "repeated_tool"
    }

    fn evaluate(
        &self,
        snapshot: &SupervisionSnapshot<'_>,
        profile: &SupervisionProfile,
    ) -> Vec<SupervisionFinding> {
        let threshold = profile.thresholds.repeated_tool_count;
        if threshold == 0 {
            return Vec::new();
        }
        let mut by_branch: HashMap<Option<String>, Vec<&RunStepSpan>> = HashMap::new();
        for span in snapshot.spans {
            if span.kind == StepSpanKind::Tool {
                by_branch
                    .entry(span.worker_run_id.clone().or_else(|| span.parent_id.clone()))
                    .or_default()
                    .push(span);
            }
        }
        let mut findings = Vec::new();
        let mut reported_tools = HashSet::new();
        for spans in by_branch.values() {
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for span in spans {
                *counts.entry(span.name.as_str()).or_default() += 1;
            }
            for (tool_name, count) in counts {
                if count >= threshold && reported_tools.insert(tool_name) {
                    findings.push(SupervisionFinding {
                        id: format!("find_rpt_{}_{}", snapshot.run_id, tool_name),
                        run_id: snapshot.run_id.into(),
                        kind: FindingKind::RepeatedTool,
                        severity: SupervisionSeverity::Warning,
                        message: format!("Tool '{tool_name}' invoked {count} times (threshold {threshold})"),
                        created_at_ms: snapshot.now_ms,
                    });
                }
            }
        }
        findings
    }
}

pub struct RepeatedErrorRule;

impl SupervisionRule for RepeatedErrorRule {
    fn rule_id(&self) -> &'static str {
        "repeated_error"
    }

    fn evaluate(
        &self,
        snapshot: &SupervisionSnapshot<'_>,
        profile: &SupervisionProfile,
    ) -> Vec<SupervisionFinding> {
        let threshold = profile.thresholds.repeated_error_count;
        if threshold == 0 {
            return Vec::new();
        }
        let mut counts: HashMap<&str, (usize, &str)> = HashMap::new();
        for err in snapshot.errors {
            let entry = counts
                .entry(err.signature_hash.as_str())
                .or_insert((0, err.error_type.as_str()));
            entry.0 += 1;
        }
        let mut findings = Vec::new();
        for (_sig, (count, err_type)) in counts {
            if count >= threshold {
                findings.push(SupervisionFinding {
                    id: format!("find_rpe_{}_{}", snapshot.run_id, err_type),
                    run_id: snapshot.run_id.into(),
                    kind: FindingKind::RepeatedError,
                    severity: SupervisionSeverity::Critical,
                    message: format!("Error '{err_type}' occurred {count} times (threshold {threshold})"),
                    created_at_ms: snapshot.now_ms,
                });
            }
        }
        findings
    }
}

pub struct NoActivityRule;

impl SupervisionRule for NoActivityRule {
    fn rule_id(&self) -> &'static str {
        "no_activity"
    }

    fn evaluate(
        &self,
        snapshot: &SupervisionSnapshot<'_>,
        profile: &SupervisionProfile,
    ) -> Vec<SupervisionFinding> {
        if !snapshot.is_active || has_active_exempt_span(snapshot, profile) {
            return Vec::new();
        }
        let threshold = profile.thresholds.no_activity_ms;
        if threshold <= 0 {
            return Vec::new();
        }
        let mut last_event_ms = snapshot.started_at_ms;
        for span in snapshot.spans {
            last_event_ms = last_event_ms
                .max(span.started_at_ms)
                .max(span.ended_at_ms.unwrap_or(span.started_at_ms));
        }
        for log in snapshot.logs {
            last_event_ms = last_event_ms.max(log.timestamp_ms);
        }
        let idle_ms = snapshot.now_ms.saturating_sub(last_event_ms);
        if idle_ms >= threshold {
            vec![SupervisionFinding {
                id: format!("find_noact_{}_{}", snapshot.run_id, snapshot.now_ms),
                run_id: snapshot.run_id.into(),
                kind: FindingKind::NoActivity,
                severity: SupervisionSeverity::Warning,
                message: format!(
                    "No execution activity for {} (threshold {})",
                    format_duration(idle_ms),
                    format_duration(threshold)
                ),
                created_at_ms: snapshot.now_ms,
            }]
        } else {
            Vec::new()
        }
    }
}

pub struct PossibleLoopRule;

impl SupervisionRule for PossibleLoopRule {
    fn rule_id(&self) -> &'static str {
        "possible_loop"
    }

    fn evaluate(
        &self,
        snapshot: &SupervisionSnapshot<'_>,
        profile: &SupervisionProfile,
    ) -> Vec<SupervisionFinding> {
        let window_size = profile.thresholds.possible_loop_window.max(4);
        let mut by_branch: BTreeMap<Option<String>, Vec<&RunStepSpan>> = BTreeMap::new();
        for span in snapshot.spans {
            if span.kind == StepSpanKind::Tool {
                by_branch
                    .entry(span.worker_run_id.clone().or_else(|| span.parent_id.clone()))
                    .or_default()
                    .push(span);
            }
        }
        let mut findings = Vec::new();
        for spans in by_branch.values() {
            let names: Vec<&str> = spans.iter().map(|s| s.name.as_str()).collect();
            let slice = if names.len() > window_size {
                &names[names.len() - window_size..]
            } else {
                &names[..]
            };
            if slice.len() >= 3 {
                let last = slice[slice.len() - 1];
                let second = slice[slice.len() - 2];
                let third = slice[slice.len() - 3];
                if last == second && second == third {
                    findings.push(SupervisionFinding {
                        id: format!("find_loop_seq_{}_{}", snapshot.run_id, last),
                        run_id: snapshot.run_id.into(),
                        kind: FindingKind::PossibleLoop,
                        severity: SupervisionSeverity::Critical,
                        message: format!("Sequential invocation loop detected on tool '{last}'"),
                        created_at_ms: snapshot.now_ms,
                    });
                    continue;
                }
            }
            if slice.len() >= 4 {
                let n = slice.len();
                if slice[n - 1] == slice[n - 3]
                    && slice[n - 2] == slice[n - 4]
                    && slice[n - 1] != slice[n - 2]
                {
                    findings.push(SupervisionFinding {
                        id: format!("find_loop_alt_{}_{}_{}", snapshot.run_id, slice[n - 2], slice[n - 1]),
                        run_id: snapshot.run_id.into(),
                        kind: FindingKind::PossibleLoop,
                        severity: SupervisionSeverity::Critical,
                        message: format!(
                            "Alternating loop pattern detected: {} -> {} -> {} -> {}",
                            slice[n - 4], slice[n - 3], slice[n - 2], slice[n - 1]
                        ),
                        created_at_ms: snapshot.now_ms,
                    });
                }
            }
        }
        findings
    }
}

/// Normalized lifecycle status of a run for supervision projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisionRunStatus {
    Running,
    Success,
    Error,
}

impl SupervisionRunStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Error => "error",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "success" | "completed" | "ok" => Self::Success,
            "error" | "failed" | "cancelled" | "canceled" => Self::Error,
            _ => Self::Running,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Projection row for a supervised run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupervisionRunRecord {
    pub run_id: String,
    pub workbench_id: Option<String>,
    pub agent: String,
    pub status: SupervisionRunStatus,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub usage_measurement: UsageMeasurement,
}

/// Consolidated Run summary row for global and per-workbench tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSummaryRow {
    pub run_id: String,
    pub task_id: Option<String>,
    pub session_id: Option<String>,
    pub workspace_id: Option<String>,
    pub workbench_id: Option<String>,
    pub agent: String,
    pub model: Option<String>,
    pub status: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub tokens: u64,
    pub cost_microcents: u64,
    pub finding_count: usize,
    pub error_count: usize,
}

/// Full inspection report for a selected run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunInspectionReport {
    pub run: RunSummaryRow,
    pub latency: LatencyBreakdown,
    pub spans: Vec<RunStepSpan>,
    pub logs: Vec<SupervisionLogEntry>,
    pub errors: Vec<StepError>,
    pub findings: Vec<SupervisionFinding>,
}

/// Fleet-wide metrics summary for the top supervision strip.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RunMetricsSummary {
    pub active_runs: usize,
    pub total_runs: usize,
    pub successful_runs: usize,
    pub failed_runs: usize,
    pub success_rate: f64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: i64,
    pub total_tokens: u64,
    pub total_cost_microcents: u64,
    pub total_findings: usize,
    pub total_errors: usize,
}

/// High-level overview combining active counts, recent runs, top errors, and slow runs.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RunOverviewSummary {
    pub metrics: RunMetricsSummary,
    pub active_runs: Vec<RunSummaryRow>,
    pub recent_runs: Vec<RunSummaryRow>,
    pub top_errors: Vec<super::finding::ErrorGroupSummary>,
    pub slow_runs: Vec<RunSummaryRow>,
    pub active_findings: Vec<SupervisionFinding>,
}
