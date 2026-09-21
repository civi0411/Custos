use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let schema = include_str!("schema.sql");
    conn.execute_batch(schema)?;
    Ok(())
}
