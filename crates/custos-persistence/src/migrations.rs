use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let schema_core = include_str!("../migrations/0001_core.sql");
    conn.execute_batch(schema_core)?;

    let schema_runtime = include_str!("../migrations/0002_runtime.sql");
    conn.execute_batch(schema_runtime)?;

    let schema_authority = include_str!("../migrations/0003_authority.sql");
    conn.execute_batch(schema_authority)?;

    let schema_usage = include_str!("../migrations/0004_usage.sql");
    conn.execute_batch(schema_usage)?;

    let schema_evidence = include_str!("../migrations/0005_evidence.sql");
    conn.execute_batch(schema_evidence)?;

    let schema_session = include_str!("../migrations/0006_session_and_bridge.sql");
    conn.execute_batch(schema_session)?;

    let has_contract_column = conn
        .prepare("PRAGMA table_info(tasks)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|column| column == "contract_json");
    if !has_contract_column {
        conn.execute_batch(include_str!("../migrations/0007_task_contract.sql"))?;
    }

    conn.execute_batch(include_str!("../migrations/0008_task_events.sql"))?;
    conn.execute_batch(include_str!("../migrations/0009_outbox_and_effects.sql"))?;
    conn.execute_batch(include_str!("../migrations/0010_harden_outbox_and_effects.sql"))?;
    conn.execute_batch(include_str!("../migrations/0011_runs_and_worker_runs.sql"))?;
    conn.execute_batch(include_str!("../migrations/0012_decision_records.sql"))?;
    conn.execute_batch(include_str!("../migrations/0013_workflow_revisions.sql"))?;
    conn.execute_batch(include_str!("../migrations/0014_node_placements.sql"))?;
    conn.execute_batch(include_str!("../migrations/0015_replan_records.sql"))?;

    Ok(())
}
