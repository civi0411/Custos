use crate::migrations::run_migrations;
use custos_core_domain::DomainError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex, MutexGuard};

/// Managed thread-safe SQLite connection handle.
/// Configures WAL mode, foreign keys, and busy timeout for high reliability.
#[derive(Clone)]
pub struct DbConnection {
    conn: Arc<Mutex<Connection>>,
}

impl DbConnection {
    /// Opens an in-memory database and runs all migrations.
    pub fn open_in_memory() -> Result<Self, DomainError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            DomainError::Validation(format!("Failed to open in-memory SQLite: {}", e))
        })?;
        Self::configure_and_migrate(conn)
    }

    /// Opens a file-backed SQLite database and runs all migrations.
    pub fn open(path: &str) -> Result<Self, DomainError> {
        let conn = Connection::open(path).map_err(|e| {
            DomainError::Validation(format!("Failed to open SQLite at {}: {}", path, e))
        })?;
        Self::configure_and_migrate(conn)
    }

    /// Configures connection PRAGMAs and applies migrations.
    fn configure_and_migrate(conn: Connection) -> Result<Self, DomainError> {
        // Enforce foreign key constraints
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            "#,
        )
        .map_err(|e| DomainError::Validation(format!("Failed to apply SQLite PRAGMAs: {}", e)))?;

        run_migrations(&conn)
            .map_err(|e| DomainError::Validation(format!("Failed to run migrations: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Acquires a lock on the underlying connection with safe PoisonError handling.
    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>, DomainError> {
        self.conn.lock().map_err(|e| {
            DomainError::Validation(format!("Database connection mutex poisoned: {}", e))
        })
    }
}
