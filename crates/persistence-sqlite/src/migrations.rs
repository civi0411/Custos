use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let schema_core = include_str!("../migrations/0001_core.sql");
    conn.execute_batch(schema_core)?;

    let schema_runtime = include_str!("../migrations/0002_runtime.sql");
    conn.execute_batch(schema_runtime)?;

    let schema_authority = include_str!("../migrations/0003_authority.sql");
    conn.execute_batch(schema_authority)?;

    Ok(())
}
