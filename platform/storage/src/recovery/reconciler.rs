use crate::repositories::{
    CommandLedgerRepository, ExperimentAttemptRepository, OutboxRepository, RunRepository,
};
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Summary of records reconciled upon crash recovery / startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReconciliationSummary {
    pub outbox_reconciled: usize,
    pub runs_reconciled: usize,
    pub attempts_reconciled: usize,
    pub commands_reconciled: usize,
}

/// SADE Crash Reconciler.
/// Scans uncertain states left by ungraceful daemon termination:
/// 1. In-flight outbox dispatch intents reset to retry/uncertain.
/// 2. In-flight runs and worker runs marked interrupted/failed.
/// 3. In-flight experiment attempts fenced to uncertain.
/// 4. Orphaned command admissions expired.
pub struct CrashReconciler;

impl CrashReconciler {
    /// Reconciles in-flight outbox entries on startup.
    pub fn reconcile_outbox(outbox_repo: &OutboxRepository) -> Result<usize, DomainError> {
        outbox_repo.reconcile_on_startup()
    }

    /// Reconciles running/active runs and worker runs on startup.
    pub fn reconcile_runs(run_repo: &RunRepository) -> Result<usize, DomainError> {
        run_repo.reconcile_runs_on_startup()
    }

    /// Reconciles executing experiment attempts to Uncertain on startup.
    pub fn reconcile_attempts(
        attempt_repo: &ExperimentAttemptRepository,
    ) -> Result<usize, DomainError> {
        attempt_repo.reconcile_attempts_on_startup()
    }

    /// Reconciles orphaned in-flight commands past timeout threshold.
    pub fn reconcile_commands(
        command_repo: &CommandLedgerRepository,
        max_age_hours: u32,
    ) -> Result<usize, DomainError> {
        command_repo.reconcile_orphaned_inflight_commands(max_age_hours)
    }

    /// Executes full crash reconciliation pass across all subsystems.
    pub fn reconcile_all(
        outbox_repo: &OutboxRepository,
        run_repo: &RunRepository,
        attempt_repo: &ExperimentAttemptRepository,
        command_repo: &CommandLedgerRepository,
    ) -> Result<ReconciliationSummary, DomainError> {
        let outbox_reconciled = Self::reconcile_outbox(outbox_repo)?;
        let runs_reconciled = Self::reconcile_runs(run_repo)?;
        let attempts_reconciled = Self::reconcile_attempts(attempt_repo)?;
        let commands_reconciled = Self::reconcile_commands(command_repo, 0)?;

        Ok(ReconciliationSummary {
            outbox_reconciled,
            runs_reconciled,
            attempts_reconciled,
            commands_reconciled,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::DbConnection;

    #[test]
    fn test_crash_reconciler_reconcile_all() {
        let db = DbConnection::open_in_memory().expect("open in memory");
        let outbox_repo = OutboxRepository::new(db.clone());
        let run_repo = RunRepository::new(db.clone());
        let attempt_repo = ExperimentAttemptRepository::new(db.writer_arc());
        let command_repo = CommandLedgerRepository::new(db.clone());

        let summary = CrashReconciler::reconcile_all(
            &outbox_repo,
            &run_repo,
            &attempt_repo,
            &command_repo,
        )
        .expect("reconciliation succeeds");

        assert_eq!(summary.outbox_reconciled, 0);
        assert_eq!(summary.runs_reconciled, 0);
        assert_eq!(summary.attempts_reconciled, 0);
        assert_eq!(summary.commands_reconciled, 0);
    }
}
