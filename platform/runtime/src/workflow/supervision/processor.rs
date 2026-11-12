//! Bounded in-memory event batching, redaction, and sweeper rule evaluation.

use super::coordinator::SupervisionCoordinator;
use super::PayloadRedactor;
use custos_core::contracts::supervision::NormalizedTelemetryBatch;
use custos_domain::supervision::{
    compute_error_signature, evaluate_snapshot, RunStepSpan, StepError, SupervisionFinding,
    SupervisionLogEntry, SupervisionProfile, SupervisionRunRecord, SupervisionRunStatus,
    SupervisionSnapshot,
};
use custos_domain::{DomainError, UsageMeasurement};
use std::collections::{HashMap, HashSet};

const MAX_ACTIVE_SPANS_PER_RUN: usize = 512;
const MAX_ACTIVE_LOGS_PER_RUN: usize = 512;
const MAX_ACTIVE_ERRORS_PER_RUN: usize = 256;

pub(super) enum SupervisionCommand {
    RunState {
        run_id: String,
        workbench_id: Option<String>,
        agent: String,
        active: bool,
        started_at_ms: i64,
        ended_at_ms: Option<i64>,
    },
    Span(RunStepSpan),
    Log(SupervisionLogEntry),
    Error(StepError),
    Batch(NormalizedTelemetryBatch),
}

pub(super) struct ActiveRunWindow {
    pub(super) record: SupervisionRunRecord,
    pub(super) spans: Vec<RunStepSpan>,
    pub(super) logs: Vec<SupervisionLogEntry>,
    pub(super) errors: Vec<StepError>,
    pub(super) emitted_findings: HashSet<String>,
}

impl SupervisionCoordinator {
    pub(super) async fn process_commands(&self, cmds: Vec<SupervisionCommand>) -> Result<(), DomainError> {
        let mut runs_out = Vec::new();
        let mut spans_out = Vec::new();
        let mut logs_out = Vec::new();
        let mut errors_out = Vec::new();
        let mut touched_runs: HashSet<String> = HashSet::new();
        let mut completed_runs: Vec<ActiveRunWindow> = Vec::new();

        {
            let mut active = self
                .active_runs
                .lock()
                .map_err(|_| DomainError::Validation("Supervision active lock unavailable".into()))?;
            let was_empty = active.is_empty();

            for cmd in cmds {
                match cmd {
                    SupervisionCommand::RunState {
                        run_id,
                        workbench_id,
                        agent,
                        active: is_active,
                        started_at_ms,
                        ended_at_ms,
                    } => {
                        let status = if is_active {
                            SupervisionRunStatus::Running
                        } else {
                            SupervisionRunStatus::Success
                        };
                        let duration_ms = ended_at_ms.map(|e| e.saturating_sub(started_at_ms));
                        let entry = active.entry(run_id.clone()).or_insert_with(|| ActiveRunWindow {
                            record: SupervisionRunRecord {
                                run_id: run_id.clone(),
                                workbench_id: workbench_id.clone(),
                                agent: agent.clone(),
                                status,
                                started_at_ms,
                                ended_at_ms,
                                duration_ms,
                                tokens: None,
                                cost_usd: None,
                                usage_measurement: UsageMeasurement::Unknown,
                            },
                            spans: Vec::new(),
                            logs: Vec::new(),
                            errors: Vec::new(),
                            emitted_findings: HashSet::new(),
                        });
                        if workbench_id.is_some() {
                            entry.record.workbench_id = workbench_id;
                        }
                        if !agent.is_empty() {
                            entry.record.agent = agent;
                        }
                        entry.record.status = status;
                        entry.record.ended_at_ms = ended_at_ms;
                        entry.record.duration_ms = duration_ms;
                        runs_out.push(entry.record.clone());
                        touched_runs.insert(run_id.clone());

                        if !is_active {
                            if let Some(finished) = active.remove(&run_id) {
                                completed_runs.push(finished);
                            }
                        }
                    }
                    SupervisionCommand::Span(mut span) => {
                        let profile = self.resolve_profile_for(&active, &span.run_id);
                        span.metadata = redactor_for(&profile).redact_json(span.metadata);
                        touched_runs.insert(span.run_id.clone());
                        let window = active
                            .entry(span.run_id.clone())
                            .or_insert_with(|| empty_window(default_run_record(&span.run_id, span.started_at_ms)));
                        upsert_bounded_span(&mut window.spans, &span);
                        self.live.emit(
                            &span.run_id,
                            "step_span",
                            serde_json::to_value(&span).unwrap_or_default(),
                        );
                        spans_out.push(span);
                    }
                    SupervisionCommand::Log(mut entry) => {
                        let profile = self.resolve_profile_for(&active, &entry.run_id);
                        let redactor = redactor_for(&profile);
                        entry.message = redactor.redact_text(&entry.message);
                        entry.metadata = redactor.redact_json(entry.metadata);
                        touched_runs.insert(entry.run_id.clone());
                        let window = active
                            .entry(entry.run_id.clone())
                            .or_insert_with(|| empty_window(default_run_record(&entry.run_id, entry.timestamp_ms)));
                        push_bounded(&mut window.logs, entry.clone(), MAX_ACTIVE_LOGS_PER_RUN);
                        self.live.emit(
                            &entry.run_id,
                            "supervision_log",
                            serde_json::to_value(&entry).unwrap_or_default(),
                        );
                        logs_out.push(entry);
                    }
                    SupervisionCommand::Error(mut err) => {
                        let profile = self.resolve_profile_for(&active, &err.run_id);
                        err.message = redactor_for(&profile).redact_text(&err.message);
                        if err.signature_hash.is_empty() {
                            let (_, hash) = compute_error_signature(&err.error_type, &err.message);
                            err.signature_hash = hash;
                        }
                        touched_runs.insert(err.run_id.clone());
                        let window = active
                            .entry(err.run_id.clone())
                            .or_insert_with(|| empty_window(default_run_record(&err.run_id, err.timestamp_ms)));
                        push_bounded(&mut window.errors, err.clone(), MAX_ACTIVE_ERRORS_PER_RUN);
                        self.live.emit(
                            &err.run_id,
                            "supervision_error",
                            serde_json::to_value(&err).unwrap_or_default(),
                        );
                        errors_out.push(err);
                    }
                    SupervisionCommand::Batch(batch) => {
                        self.apply_batch(
                            batch,
                            &mut active,
                            &mut touched_runs,
                            &mut completed_runs,
                            &mut runs_out,
                            &mut spans_out,
                            &mut logs_out,
                            &mut errors_out,
                        );
                    }
                }
            }

            if was_empty && !active.is_empty() {
                self.sweeper_notify.notify_one();
            }
        }

        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut findings_out = Vec::new();
        {
            let mut active = self
                .active_runs
                .lock()
                .map_err(|_| DomainError::Validation("Supervision active lock unavailable".into()))?;
            for run_id in touched_runs {
                if let Some(window) = active.get_mut(&run_id) {
                    findings_out.extend(self.evaluate_window(window, now_ms));
                }
            }
        }

        for mut window in completed_runs {
            window.record.status = SupervisionRunStatus::Success;
            findings_out.extend(self.evaluate_window(&mut window, now_ms));
        }

        if let Some(ref store) = self.store {
            store
                .persist_batch(&runs_out, &spans_out, &logs_out, &errors_out, &findings_out)
                .await?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_batch(
        &self,
        batch: NormalizedTelemetryBatch,
        active: &mut HashMap<String, ActiveRunWindow>,
        touched_runs: &mut HashSet<String>,
        completed_runs: &mut Vec<ActiveRunWindow>,
        runs_out: &mut Vec<SupervisionRunRecord>,
        spans_out: &mut Vec<RunStepSpan>,
        logs_out: &mut Vec<SupervisionLogEntry>,
        errors_out: &mut Vec<StepError>,
    ) {
        for mut run in batch.runs {
            if run.status.is_active() {
                let entry = active
                    .entry(run.run_id.clone())
                    .or_insert_with(|| empty_window(run.clone()));
                entry.record = run.clone();
                touched_runs.insert(run.run_id.clone());
            } else if let Some(mut finished) = active.remove(&run.run_id) {
                if run.ended_at_ms.is_none() {
                    run.ended_at_ms = finished.record.ended_at_ms;
                }
                finished.record = run.clone();
                completed_runs.push(finished);
            }
            runs_out.push(run);
        }
        for mut span in batch.spans {
            let profile = self.resolve_profile_for(active, &span.run_id);
            span.metadata = redactor_for(&profile).redact_json(span.metadata);
            touched_runs.insert(span.run_id.clone());
            if let Some(w) = active.get_mut(&span.run_id) {
                upsert_bounded_span(&mut w.spans, &span);
            }
            spans_out.push(span);
        }
        for mut log in batch.logs {
            let profile = self.resolve_profile_for(active, &log.run_id);
            let redactor = redactor_for(&profile);
            log.message = redactor.redact_text(&log.message);
            log.metadata = redactor.redact_json(log.metadata);
            if let Some(w) = active.get_mut(&log.run_id) {
                push_bounded(&mut w.logs, log.clone(), MAX_ACTIVE_LOGS_PER_RUN);
            }
            logs_out.push(log);
        }
        for mut err in batch.errors {
            let profile = self.resolve_profile_for(active, &err.run_id);
            err.message = redactor_for(&profile).redact_text(&err.message);
            if err.signature_hash.is_empty() {
                let (_, h) = compute_error_signature(&err.error_type, &err.message);
                err.signature_hash = h;
            }
            touched_runs.insert(err.run_id.clone());
            if let Some(w) = active.get_mut(&err.run_id) {
                push_bounded(&mut w.errors, err.clone(), MAX_ACTIVE_ERRORS_PER_RUN);
            }
            errors_out.push(err);
        }
    }

    pub(super) async fn sweep_active_runs(&self) -> Result<(), DomainError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut new_findings = Vec::new();
        {
            let mut active = self
                .active_runs
                .lock()
                .map_err(|_| DomainError::Validation("Supervision active lock unavailable".into()))?;
            for window in active.values_mut() {
                new_findings.extend(self.evaluate_window(window, now_ms));
            }
        }
        if !new_findings.is_empty() {
            if let Some(ref store) = self.store {
                store
                    .persist_batch(&[], &[], &[], &[], &new_findings)
                    .await?;
            }
        }
        Ok(())
    }

    fn evaluate_window(
        &self,
        window: &mut ActiveRunWindow,
        now_ms: i64,
    ) -> Vec<SupervisionFinding> {
        let profile = self
            .profiles
            .resolve(window.record.workbench_id.as_deref());
        let snap = SupervisionSnapshot {
            run_id: &window.record.run_id,
            agent: &window.record.agent,
            is_active: window.record.status.is_active(),
            started_at_ms: window.record.started_at_ms,
            ended_at_ms: window.record.ended_at_ms,
            duration_ms: window.record.duration_ms,
            spans: &window.spans,
            logs: &window.logs,
            errors: &window.errors,
            now_ms,
        };
        let mut emitted = Vec::new();
        for finding in evaluate_snapshot(&self.rules, &snap, &profile) {
            if window.emitted_findings.insert(finding.id.clone()) {
                self.live.emit(
                    &finding.run_id,
                    "supervision_finding",
                    serde_json::to_value(&finding).unwrap_or_default(),
                );
                emitted.push(finding);
            }
        }
        emitted
    }

    fn resolve_profile_for(
        &self,
        active: &HashMap<String, ActiveRunWindow>,
        run_id: &str,
    ) -> SupervisionProfile {
        let wb = active
            .get(run_id)
            .and_then(|w| w.record.workbench_id.as_deref());
        self.profiles.resolve(wb)
    }
}

fn empty_window(record: SupervisionRunRecord) -> ActiveRunWindow {
    ActiveRunWindow {
        record,
        spans: Vec::new(),
        logs: Vec::new(),
        errors: Vec::new(),
        emitted_findings: HashSet::new(),
    }
}

fn default_run_record(run_id: &str, started_at_ms: i64) -> SupervisionRunRecord {
    SupervisionRunRecord {
        run_id: run_id.to_string(),
        workbench_id: None,
        agent: "custos-runtime".into(),
        status: SupervisionRunStatus::Running,
        started_at_ms,
        ended_at_ms: None,
        duration_ms: None,
        tokens: None,
        cost_usd: None,
        usage_measurement: UsageMeasurement::Unknown,
    }
}

fn redactor_for(profile: &SupervisionProfile) -> PayloadRedactor {
    PayloadRedactor {
        enabled: profile.redaction_enabled,
        max_payload_bytes: profile.max_payload_bytes,
    }
}

fn push_bounded<T>(vec: &mut Vec<T>, item: T, max_len: usize) {
    if vec.len() >= max_len {
        vec.remove(0);
    }
    vec.push(item);
}

fn upsert_bounded_span(spans: &mut Vec<RunStepSpan>, incoming: &RunStepSpan) {
    if let Some(existing) = spans.iter_mut().find(|s| s.id == incoming.id) {
        *existing = incoming.clone();
        return;
    }
    push_bounded(spans, incoming.clone(), MAX_ACTIVE_SPANS_PER_RUN);
}

