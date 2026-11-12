//! Storage Reconciliation on Startup (SADE R8 §6.8)

use custos_domain::DomainError;
use custos_persistence::SqliteTaskStore;

/// Performs crash recovery and reconciles orphaned runs and inflight commands on daemon startup.
pub fn reconcile_storage_on_startup(store: &SqliteTaskStore) -> Result<(), DomainError> {
    // Crash Recovery Reconcile (Gate 3): transition any InFlight effects to Uncertain on startup
    store.outbox().reconcile_on_startup()?;
    store.reconcile_runs_on_startup()?;
    let _ = store
        .command_ledger()
        .reconcile_orphaned_inflight_commands(24);
    Ok(())
}
