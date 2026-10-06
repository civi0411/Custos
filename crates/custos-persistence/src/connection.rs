use crate::migrations::run_migrations;
use custos_domain::DomainError;
use rusqlite::{Connection, OpenFlags};
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};

const DEFAULT_POOL_MAX_SIZE: usize = 16;

/// Managed thread-safe SQLite connection handle with dedicated WAL Reader Pool (P0).
/// Writes acquire the single sovereign writer lock.
/// Reads acquire independent read-only connections from the reader pool,
/// enabling high-concurrency read fan-out without serializing workers.
#[derive(Clone)]
pub struct DbConnection {
    writer: Arc<Mutex<Connection>>,
    reader_pool: Arc<ReaderPool>,
}

pub struct ReaderPool {
    uri: String,
    is_memory: bool,
    idle: Mutex<Vec<Connection>>,
    max_size: usize,
}

pub struct PooledReader {
    conn: Option<Connection>,
    pool: Arc<ReaderPool>,
}

impl Deref for PooledReader {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        self.conn
            .as_ref()
            .expect("Connection available in PooledReader")
    }
}

impl Drop for PooledReader {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if let Ok(mut idle) = self.pool.idle.lock() {
                if idle.len() < self.pool.max_size {
                    idle.push(conn);
                }
            }
        }
    }
}

impl DbConnection {
    /// Opens an in-memory database with shared cache for reader pool and runs all migrations.
    pub fn open_in_memory() -> Result<Self, DomainError> {
        let mem_id = uuid::Uuid::new_v4().simple();
        let uri = format!("file:custos_mem_{}?mode=memory&cache=shared", mem_id);

        let writer = Connection::open_with_flags(
            &uri,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI,
        )
        .map_err(|e| {
            DomainError::Validation(format!("Failed to open shared in-memory SQLite: {}", e))
        })?;

        Self::configure_and_init(writer, uri, true)
    }

    /// Opens a file-backed SQLite database in WAL mode and runs all migrations.
    pub fn open(path: &str) -> Result<Self, DomainError> {
        let writer = Connection::open(path).map_err(|e| {
            DomainError::Validation(format!("Failed to open SQLite at {}: {}", path, e))
        })?;

        Self::configure_and_init(writer, path.to_string(), false)
    }

    /// Configures connection PRAGMAs, runs migrations on writer, and initializes reader pool.
    fn configure_and_init(
        writer: Connection,
        uri: String,
        is_memory: bool,
    ) -> Result<Self, DomainError> {
        // Enforce foreign key constraints and WAL mode
        writer
            .execute_batch(
                r#"
                PRAGMA foreign_keys = ON;
                PRAGMA journal_mode = WAL;
                PRAGMA busy_timeout = 5000;
                PRAGMA synchronous = NORMAL;
                "#,
            )
            .map_err(|e| {
                DomainError::Validation(format!("Failed to apply SQLite PRAGMAs: {}", e))
            })?;

        run_migrations(&writer)
            .map_err(|e| DomainError::Validation(format!("Failed to run migrations: {}", e)))?;

        let reader_pool = Arc::new(ReaderPool {
            uri,
            is_memory,
            idle: Mutex::new(Vec::new()),
            max_size: DEFAULT_POOL_MAX_SIZE,
        });

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            reader_pool,
        })
    }

    /// Acquires a lock on the underlying sovereign writer connection.
    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>, DomainError> {
        self.writer
            .lock()
            .map_err(|e| DomainError::Validation(format!("Database writer mutex poisoned: {}", e)))
    }

    /// Acquires a pooled reader connection from the WAL reader pool.
    pub fn reader(&self) -> Result<PooledReader, DomainError> {
        // Try to pop from idle pool
        {
            let mut idle = self.reader_pool.idle.lock().map_err(|e| {
                DomainError::Validation(format!("Reader pool mutex poisoned: {}", e))
            })?;
            if let Some(conn) = idle.pop() {
                return Ok(PooledReader {
                    conn: Some(conn),
                    pool: self.reader_pool.clone(),
                });
            }
        }

        // Open a new reader connection
        let conn = if self.reader_pool.is_memory {
            Connection::open_with_flags(
                &self.reader_pool.uri,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_URI,
            )
        } else {
            Connection::open_with_flags(
                &self.reader_pool.uri,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            )
        }
        .map_err(|e| DomainError::Validation(format!("Failed to open reader connection: {}", e)))?;

        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA query_only = ON;
            "#,
        )
        .map_err(|e| {
            DomainError::Validation(format!("Failed to configure reader PRAGMAs: {}", e))
        })?;

        Ok(PooledReader {
            conn: Some(conn),
            pool: self.reader_pool.clone(),
        })
    }

    /// Runs a read-only closure using a pooled reader connection.
    pub fn read<F, R>(&self, f: F) -> Result<R, DomainError>
    where
        F: FnOnce(&Connection) -> Result<R, DomainError>,
    {
        let reader = self.reader()?;
        f(&reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[test]
    fn test_concurrent_readers_in_memory() {
        let db = DbConnection::open_in_memory().expect("open in memory");

        // Insert test data via writer
        {
            let conn = db.lock().expect("writer lock");
            conn.execute(
                "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at)
                 VALUES ('task_pool_1', 'Pool Test', 'pending', 1, '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
                [],
            )
            .expect("insert task");
        }

        // Concurrently read via 8 worker threads using reader pool
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..8 {
            let db_clone = db.clone();
            let count_clone = success_count.clone();
            handles.push(thread::spawn(move || {
                let reader = db_clone.reader().expect("acquire reader");
                let mut stmt = reader
                    .prepare("SELECT title FROM tasks WHERE id = 'task_pool_1'")
                    .unwrap();
                let title: String = stmt.query_row([], |row| row.get(0)).unwrap();
                if title == "Pool Test" {
                    count_clone.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(success_count.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn test_concurrent_readers_file_wal() {
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        let path = tmp.path().to_str().expect("valid path");

        let db = DbConnection::open(path).expect("open file db");

        // Insert test data via writer
        {
            let conn = db.lock().expect("writer lock");
            conn.execute(
                "INSERT INTO tasks (id, title, status, epoch, created_at, updated_at)
                 VALUES ('task_wal_1', 'WAL Test', 'pending', 1, '2026-10-04T00:00:00Z', '2026-10-04T00:00:00Z')",
                [],
            )
            .expect("insert task");
        }

        // Concurrently read via 8 worker threads while writer performs updates
        let success_count = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..8 {
            let db_clone = db.clone();
            let count_clone = success_count.clone();
            handles.push(thread::spawn(move || {
                let reader = db_clone.reader().expect("acquire reader");
                let mut stmt = reader
                    .prepare("SELECT title FROM tasks WHERE id = 'task_wal_1'")
                    .unwrap();
                let title: String = stmt.query_row([], |row| row.get(0)).unwrap();
                if title == "WAL Test" {
                    count_clone.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(success_count.load(Ordering::SeqCst), 8);
    }
}
