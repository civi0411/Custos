use custos_contracts::DomainError;
use rusqlite::Connection;

/// Applies sovereign WAL writer PRAGMAs: foreign keys ON, WAL mode, busy timeout 5s, NORMAL synchronous.
pub fn apply_writer_pragmas(writer: &Connection) -> Result<(), DomainError> {
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
            DomainError::Validation(format!("Failed to apply SQLite writer PRAGMAs: {}", e))
        })
}

/// Applies read-only PRAGMAs: foreign keys ON, busy timeout 5s, query_only ON.
pub fn apply_reader_pragmas(reader: &Connection) -> Result<(), DomainError> {
    reader
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA query_only = ON;
            "#,
        )
        .map_err(|e| DomainError::Validation(format!("Failed to configure reader PRAGMAs: {}", e)))
}
