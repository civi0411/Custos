//! Workflow Task Scheduler
//!
//! Manages time-based, interval, and recurring workflow job scheduling.

use chrono::{DateTime, Duration, Utc};
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
        }
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
                JobRecurrence::Cron { .. } => {
                    // For cron without full parser crate, advance by standard 1 hour fallback or reschedule
                    if success {
                        job.status = JobStatus::Scheduled;
                        job.next_run_at = Utc::now() + Duration::hours(1);
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
}
