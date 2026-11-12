pub mod claims;
pub mod executions;
pub mod sources;
pub mod synthesis;

#[allow(unused_imports)]
pub use claims::*;
#[allow(unused_imports)]
pub use executions::*;
#[allow(unused_imports)]
pub use sources::*;
#[allow(unused_imports)]
pub use synthesis::*;

use std::sync::{Arc, Mutex};
use custos_domain::DomainError;

#[derive(Clone)]
pub struct DbConnection(pub Arc<Mutex<rusqlite::Connection>>);

impl DbConnection {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self(conn)
    }

    pub fn open(path: &str) -> Result<Self, DomainError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn open_in_memory() -> Result<Self, DomainError> {
        let conn = rusqlite::Connection::open_in_memory()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, DomainError> {
        self.0
            .lock()
            .map_err(|e| DomainError::Validation(format!("Database mutex poisoned: {e}")))
    }
}

#[derive(Clone)]
pub struct ResearchRepository {
    pub(crate) db: DbConnection,
}

impl ResearchRepository {
    pub fn new(db: DbConnection) -> Self {
        let repo = Self { db };
        if let Err(e) = repo.ensure_tables() {
            tracing::error!("Failed to initialize research schema: {e}");
        }
        repo
    }

    pub fn from_raw(writer: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self::new(DbConnection::new(writer))
    }

    fn ensure_column(
        conn: &rusqlite::Connection,
        table: &str,
        col: &str,
        col_type: &str,
    ) -> Result<(), DomainError> {
        let pragma_sql = format!("PRAGMA table_info({table})");
        let mut stmt = conn
            .prepare(&pragma_sql)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let exists = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .filter_map(|r| r.ok())
            .any(|c| c.eq_ignore_ascii_case(col));
        if !exists {
            conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {col} {col_type};"))
                .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        Ok(())
    }

    pub fn ensure_tables(&self) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute_batch(include_str!("../../migrations/research_schema.sql"))
            .map_err(|e| DomainError::Validation(format!("Failed to initialize research schema: {e}")))?;
        Self::ensure_column(&conn, "research_execution_records", "record_json", "TEXT")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests_checkpoint;
#[cfg(test)]
mod tests_lifecycle;
