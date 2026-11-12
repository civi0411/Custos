//! Ring 2 Shared Capabilities Repository
//!
//! Persists cross-workbench artifacts (lineage & annotations), versioned notes,
//! interactive notebook cells/kernels, and independent reviewer records in SQLite WAL.

mod annotations;
mod artifacts;
mod notebooks;
mod notes;
mod reviews;

#[cfg(test)]
mod tests;

use crate::connection::DbConnection;
use custos_domain::DomainError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex, MutexGuard};

/// SQLite-backed persistence repository for Ring 2 Shared Capabilities.
#[derive(Clone)]
pub struct SharedCapabilitiesRepository {
    pub(crate) db: Arc<Mutex<Connection>>,
}

impl SharedCapabilitiesRepository {
    pub fn new(db: DbConnection) -> Self {
        Self {
            db: db.writer_arc(),
        }
    }

    pub fn from_raw(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    pub fn raw_db(&self) -> Arc<Mutex<Connection>> {
        self.db.clone()
    }

    /// Applies the Ring 2 schema idempotently for standalone raw connections.
    pub fn ensure_schema(&self) -> Result<(), DomainError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(include_str!("../../../migrations/0045_shared_capabilities.sql"))
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub(crate) fn lock_conn(&self) -> Result<MutexGuard<'_, Connection>, DomainError> {
        self.db
            .lock()
            .map_err(|e| DomainError::Validation(format!("Database writer mutex poisoned: {e}")))
    }
}

