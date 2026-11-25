use crate::connection::DbConnection;
use custos_contracts::DomainError;
use rusqlite::Transaction;

/// UnitOfWork wraps a transactional boundary (T1–T5) in SQLite WAL.
/// Ensures atomic commits across state transitions, domain events, and outbox intents.
pub struct UnitOfWork;

impl UnitOfWork {
    /// Executes a closure inside an immediate SQLite transaction, committing on Ok and rolling back on Err.
    pub fn execute<T, F>(db: &DbConnection, f: F) -> Result<T, DomainError>
    where
        F: FnOnce(&Transaction) -> Result<T, DomainError>,
    {
        let mut conn = db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(format!("Failed to begin transaction: {}", e)))?;

        let result = f(&tx)?;

        tx.commit()
            .map_err(|e| DomainError::Validation(format!("Failed to commit transaction: {}", e)))?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_of_work_commit_and_rollback() {
        let db = DbConnection::open_in_memory().expect("open in memory");

        // Successful transaction commit
        UnitOfWork::execute(&db, |tx| {
            tx.execute(
                "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at)
                 VALUES ('uow_task_1', 'UOW Task', 'pending', 1, '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
                [],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
            Ok(())
        })
        .expect("uow commit succeeds");

        // Verify row exists
        let exists = db
            .read(|conn| {
                let mut stmt = conn
                    .prepare("SELECT count(*) FROM tasks WHERE id = 'uow_task_1'")
                    .map_err(|e| DomainError::Validation(e.to_string()))?;
                let count: i64 = stmt
                    .query_row([], |row| row.get(0))
                    .map_err(|e| DomainError::Validation(e.to_string()))?;
                Ok(count == 1)
            })
            .expect("read count");
        assert!(exists);

        // Rolled back transaction on Err
        let err_res: Result<(), DomainError> = UnitOfWork::execute(&db, |tx| {
            tx.execute(
                "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at)
                 VALUES ('uow_task_2', 'UOW Task 2', 'pending', 1, '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
                [],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
            Err(DomainError::Validation("Simulated failure".into()))
        });
        assert!(err_res.is_err());

        // Verify row was rolled back and does not exist
        let exists_2 = db
            .read(|conn| {
                let mut stmt = conn
                    .prepare("SELECT count(*) FROM tasks WHERE id = 'uow_task_2'")
                    .map_err(|e| DomainError::Validation(e.to_string()))?;
                let count: i64 = stmt
                    .query_row([], |row| row.get(0))
                    .map_err(|e| DomainError::Validation(e.to_string()))?;
                Ok(count == 1)
            })
            .expect("read count");
        assert!(!exists_2);
    }
}
