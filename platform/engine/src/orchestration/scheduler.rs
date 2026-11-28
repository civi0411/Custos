//! Workflow Task Scheduler
//!
//! Manages time-based, interval, and recurring workflow job scheduling.

use chrono::{DateTime, Duration, Utc};
use custos_contracts::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeSettlePolicy {
    pub settle_ms: i64,
    pub clock_jump_threshold_ms: i64,
}

impl Default for WakeSettlePolicy {
    fn default() -> Self {
        Self {
            settle_ms: 15_000,
            clock_jump_threshold_ms: 60_000,
        }
    }
}

/// Defers due job dispatch briefly after a system sleep/wake clock jump.
pub struct WakeSettleGate {
    policy: WakeSettlePolicy,
    last_tick_ms: AtomicI64,
    settled_after_ms: AtomicI64,
}

impl WakeSettleGate {
    pub fn new(policy: WakeSettlePolicy, now_ms: i64) -> Self {
        Self {
            policy,
            last_tick_ms: AtomicI64::new(now_ms),
            settled_after_ms: AtomicI64::new(now_ms),
        }
    }

    /// Returns true when settled; false if a wake jump was detected and settling is in progress.
    pub fn check_settled(&self, now_ms: i64) -> bool {
        let prev = self.last_tick_ms.swap(now_ms, Ordering::SeqCst);
        if now_ms - prev > self.policy.clock_jump_threshold_ms {
            self.settled_after_ms
                .store(now_ms + self.policy.settle_ms, Ordering::SeqCst);
            return false;
        }
        now_ms >= self.settled_after_ms.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScheduledJobId(pub String);

impl ScheduledJobId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(format!("job_{}", uuid::Uuid::new_v4().simple()))
    }
}

impl std::fmt::Display for ScheduledJobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<custos_contracts::JobId> for ScheduledJobId {
    fn from(id: custos_contracts::JobId) -> Self {
        Self(id.0)
    }
}

impl From<ScheduledJobId> for custos_contracts::JobId {
    fn from(id: ScheduledJobId) -> Self {
        Self(id.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobRecurrence {
    OneShot,
    Interval { seconds: u64 },
    Cron { expression: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Scheduled,
    Running,
    Completed,
    Failed { reason: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: ScheduledJobId,
    pub name: String,
    pub recurrence: JobRecurrence,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub status: JobStatus,
    pub contract_payload: Value,
}

pub struct WorkflowScheduler {
    jobs: Arc<RwLock<HashMap<ScheduledJobId, ScheduledJob>>>,
    generation: Arc<AtomicU64>,
    lanes: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    wake_gate: WakeSettleGate,
}

impl Default for WorkflowScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            generation: Arc::new(AtomicU64::new(1)),
            lanes: Arc::new(Mutex::new(HashMap::new())),
            wake_gate: WakeSettleGate::new(WakeSettlePolicy::default(), Utc::now().timestamp_millis()),
        }
    }

    pub fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn bump_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn check_wake_settled(&self, now_ms: i64) -> bool {
        self.wake_gate.check_settled(now_ms)
    }

    /// Serializes concurrent executions for the same job key.
    pub async fn lane_for(&self, job_key: &str) -> Arc<Mutex<()>> {
        let mut guard = self.lanes.lock().await;
        guard
            .entry(job_key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Schedule a new workflow job
    pub async fn schedule(
        &self,
        name: impl Into<String>,
        recurrence: JobRecurrence,
        first_run_at: DateTime<Utc>,
        payload: Value,
    ) -> ScheduledJobId {
        let id = ScheduledJobId::generate();
        let job = ScheduledJob {
            id: id.clone(),
            name: name.into(),
            recurrence,
            next_run_at: first_run_at,
            last_run_at: None,
            status: JobStatus::Scheduled,
            contract_payload: payload,
        };

        let mut lock = self.jobs.write().await;
        lock.insert(id.clone(), job);
        id
    }

    /// Retrieve a job by its scheduled job ID
    pub async fn get_job(&self, id: &ScheduledJobId) -> Option<ScheduledJob> {
        let lock = self.jobs.read().await;
        lock.get(id).cloned()
    }

    /// Retrieve all jobs that are due for execution at the specified reference time
    pub async fn get_due_jobs(&self, now: DateTime<Utc>) -> Vec<ScheduledJob> {
        let lock = self.jobs.read().await;
        lock.values()
            .filter(|job| job.status == JobStatus::Scheduled && job.next_run_at <= now)
            .cloned()
            .collect()
    }

    /// Mark a job as running
    pub async fn mark_running(&self, id: &ScheduledJobId) -> Result<(), DomainError> {
        let mut lock = self.jobs.write().await;
        if let Some(job) = lock.get_mut(id) {
            job.status = JobStatus::Running;
            job.last_run_at = Some(Utc::now());
            Ok(())
        } else {
            Err(DomainError::NotFound {
                kind: "ScheduledJob".into(),
                id: id.to_string(),
            })
        }
    }

    /// Mark a job as finished, recalculating the next occurrence for recurring jobs
    pub async fn mark_finished(
        &self,
        id: &ScheduledJobId,
        success: bool,
        error_message: Option<String>,
    ) -> Result<(), DomainError> {
        let mut lock = self.jobs.write().await;
        if let Some(job) = lock.get_mut(id) {
            match &job.recurrence {
                JobRecurrence::OneShot => {
                    job.status = if success {
                        JobStatus::Completed
                    } else {
                        JobStatus::Failed {
                            reason: error_message.unwrap_or_else(|| "Unknown failure".into()),
                        }
                    };
                }
                JobRecurrence::Interval { seconds } => {
                    if success {
                        job.status = JobStatus::Scheduled;
                        job.next_run_at = Utc::now() + Duration::seconds(*seconds as i64);
                    } else {
                        job.status = JobStatus::Failed {
                            reason: error_message.unwrap_or_else(|| "Recurrence error".into()),
                        };
                    }
                }
                JobRecurrence::Cron { expression } => {
                    if success {
                        job.status = JobStatus::Scheduled;
                        job.next_run_at = next_cron_run(expression, Utc::now());
                    } else {
                        job.status = JobStatus::Failed {
                            reason: error_message.unwrap_or_else(|| "Cron step failed".into()),
                        };
                    }
                }
            }
            Ok(())
        } else {
            Err(DomainError::NotFound {
                kind: "ScheduledJob".into(),
                id: id.to_string(),
            })
        }
    }

    /// Cancel a scheduled job
    pub async fn cancel(&self, id: &ScheduledJobId) -> Result<(), DomainError> {
        let mut lock = self.jobs.write().await;
        if let Some(job) = lock.get_mut(id) {
            job.status = JobStatus::Cancelled;
            Ok(())
        } else {
            Err(DomainError::NotFound {
                kind: "ScheduledJob".into(),
                id: id.to_string(),
            })
        }
    }
}

fn cron_field_matches(field: &str, value: u32, min: u32, max: u32) -> bool {
    for part in field.split(',') {
        let part = part.trim();
        let (range_part, step) = if let Some((r, s)) = part.split_once('/') {
            let Ok(step_val) = s.parse::<u32>() else {
                return false;
            };
            if step_val == 0 {
                return false;
            }
            (r, step_val)
        } else {
            (part, 1)
        };
        let (start, end) = if range_part == "*" {
            (min, max)
        } else if let Some((a, b)) = range_part.split_once('-') {
            let (Ok(sa), Ok(sb)) = (a.parse::<u32>(), b.parse::<u32>()) else {
                return false;
            };
            (sa, sb)
        } else {
            let Ok(exact) = range_part.parse::<u32>() else {
                return false;
            };
            if step == 1 {
                if value == exact {
                    return true;
                }
                continue;
            }
            (exact, max)
        };
        if value >= start && value <= end && (value - start) % step == 0 {
            return true;
        }
    }
    false
}

/// Evaluates a 5-field UTC cron expression (`min hour dom month dow`) after `after`.
/// Falls back to `after + 1 hour` only if the expression is malformed.
pub fn next_cron_run(expr: &str, after: DateTime<Utc>) -> DateTime<Utc> {
    use chrono::{Datelike, Timelike};
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return after + Duration::hours(1);
    }
    let mut candidate = after + Duration::minutes(1);
    candidate = candidate
        .with_second(0)
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(candidate);

    // Bound scan to 7 days of minutes (10,080 iterations) for deterministic O(1) runtime.
    for _ in 0..10_080 {
        let m_ok = cron_field_matches(fields[0], candidate.minute(), 0, 59);
        let h_ok = cron_field_matches(fields[1], candidate.hour(), 0, 23);
        let dom_ok = cron_field_matches(fields[2], candidate.day(), 1, 31);
        let mon_ok = cron_field_matches(fields[3], candidate.month(), 1, 12);
        let dow = candidate.weekday().num_days_from_sunday();
        let dow_ok = cron_field_matches(fields[4], dow, 0, 6);
        if m_ok && h_ok && dom_ok && mon_ok && dow_ok {
            return candidate;
        }
        candidate += Duration::minutes(1);
    }
    after + Duration::hours(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_oneshot_job_lifecycle() {
        let scheduler = WorkflowScheduler::new();
        let past = Utc::now() - Duration::seconds(10);

        let job_id = scheduler
            .schedule(
                "HealthCheck",
                JobRecurrence::OneShot,
                past,
                json!({"check": "db"}),
            )
            .await;

        let due = scheduler.get_due_jobs(Utc::now()).await;
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, job_id);

        scheduler.mark_running(&job_id).await.unwrap();
        scheduler.mark_finished(&job_id, true, None).await.unwrap();

        // No more due jobs
        let due_after = scheduler.get_due_jobs(Utc::now()).await;
        assert!(due_after.is_empty());
    }

    #[tokio::test]
    async fn test_interval_job_rescheduling() {
        let scheduler = WorkflowScheduler::new();
        let past = Utc::now() - Duration::seconds(5);

        let job_id = scheduler
            .schedule(
                "RecurringTelemetry",
                JobRecurrence::Interval { seconds: 300 },
                past,
                json!({}),
            )
            .await;

        let due = scheduler.get_due_jobs(Utc::now()).await;
        assert_eq!(due.len(), 1);

        scheduler.mark_running(&job_id).await.unwrap();
        scheduler.mark_finished(&job_id, true, None).await.unwrap();

        // Next run should be in the future (~300s)
        let due_now = scheduler.get_due_jobs(Utc::now()).await;
        assert!(due_now.is_empty());

        let future = Utc::now() + Duration::seconds(305);
        let due_future = scheduler.get_due_jobs(future).await;
        assert_eq!(due_future.len(), 1);
    }

    #[tokio::test]
    async fn test_wake_settle_gate_generation_and_lanes() {
        let scheduler = WorkflowScheduler::new();
        assert_eq!(scheduler.current_generation(), 1);
        assert_eq!(scheduler.bump_generation(), 2);

        let base_ms = Utc::now().timestamp_millis();
        assert!(scheduler.check_wake_settled(base_ms));
        // Simulate laptop sleep jump (+120s > 60s threshold)
        assert!(!scheduler.check_wake_settled(base_ms + 120_000));
        // Still settling at +125s (< +135s)
        assert!(!scheduler.check_wake_settled(base_ms + 125_000));
        // Settled after 15s
        assert!(scheduler.check_wake_settled(base_ms + 136_000));

        let lane_a1 = scheduler.lane_for("job_1").await;
        let lane_a2 = scheduler.lane_for("job_1").await;
        assert!(Arc::ptr_eq(&lane_a1, &lane_a2));
    }

    #[test]
    fn test_cron_next_run_computes_real_schedule() {
        let base = chrono::DateTime::parse_from_rfc3339("2026-11-28T10:12:30Z")
            .unwrap()
            .with_timezone(&Utc);
        let next = next_cron_run("*/15 * * * *", base);
        assert_eq!(next.to_rfc3339(), "2026-11-28T10:15:00+00:00");

        let daily = next_cron_run("30 14 * * *", base);
        assert_eq!(daily.to_rfc3339(), "2026-11-28T14:30:00+00:00");
    }
}
