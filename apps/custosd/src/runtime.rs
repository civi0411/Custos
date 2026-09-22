//! Custos Composition Root & Runtime Initialization
//!
//! Wires together the persistence store, policy engine, gateway, and services.

use custos_authority_engine::AuthorityEngine;
use custos_capability_gateway::DeterministicGate;
use custos_core_domain::DomainError;
use custos_persistence_sqlite::SqliteTaskStore;
use custos_task_kernel::{SpanService, TaskService};
use std::sync::Arc;

#[allow(dead_code)]
pub struct CustosRuntime {
    pub task_service: Arc<TaskService>,
    pub span_service: Arc<SpanService>,
    pub authority: Arc<AuthorityEngine>,
    pub gateway: Arc<DeterministicGate>,
}

impl CustosRuntime {
    pub fn bootstrap_in_memory() -> Result<Self, DomainError> {
        let store = Arc::new(SqliteTaskStore::new_in_memory()?);
        let task_service = Arc::new(TaskService::new(store.clone()));
        let span_service = Arc::new(SpanService::new(store));
        let authority = Arc::new(AuthorityEngine::default());
        let gateway = Arc::new(DeterministicGate::new(authority.clone()));

        Ok(Self {
            task_service,
            span_service,
            authority,
            gateway,
        })
    }
}
