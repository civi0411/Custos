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
    conn.execute_batch(include_str!(
        "../migrations/0010_harden_outbox_and_effects.sql"
    ))?;
    conn.execute_batch(include_str!("../migrations/0011_runs_and_worker_runs.sql"))?;
    conn.execute_batch(include_str!("../migrations/0012_decision_records.sql"))?;
    conn.execute_batch(include_str!("../migrations/0013_workflow_revisions.sql"))?;
    conn.execute_batch(include_str!("../migrations/0014_node_placements.sql"))?;
    conn.execute_batch(include_str!("../migrations/0015_replan_records.sql"))?;
    conn.execute_batch(include_str!(
        "../migrations/0016_execution_workspaces.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0017_research_lineage_and_claims.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0018_research_recipes_and_annotations.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0019_providers_and_keys.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0020_notes_and_artifact_registry.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0021_notebook_cells_and_kernels.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0022_reviewer_records.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0023_research_synthesis_proposals.sql"
    ))?;
    conn.execute_batch(include_str!(
        "../migrations/0024_browser_fleet_automation.sql"
    ))?;

    let existing_provider_cols = conn
        .prepare("PRAGMA table_info(providers_config)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if !existing_provider_cols.iter().any(|c| c == "default_model") {
        let _ = conn.execute("ALTER TABLE providers_config ADD COLUMN default_model TEXT", []);
    }
    if !existing_provider_cols.iter().any(|c| c == "context_window") {
        let _ = conn.execute("ALTER TABLE providers_config ADD COLUMN context_window INTEGER", []);
    }
    if !existing_provider_cols.iter().any(|c| c == "fast_mode") {
        let _ = conn.execute("ALTER TABLE providers_config ADD COLUMN fast_mode INTEGER", []);
    }

    conn.execute_batch(include_str!(
        "../migrations/0025_provider_model_catalog.sql"
    ))?;

    Ok(())
}
