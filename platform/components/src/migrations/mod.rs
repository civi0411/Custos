//! Pillar 3: Dynamic Workbench Database Migrations
//!
//! Dispatches and applies domain-specific SQL migrations dynamically when
//! workbenches are installed (e.g. `workbenches/{assistant,coding,quant,research}/migrations`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationScript {
    pub name: String,
    pub path: PathBuf,
    pub sql: String,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchMigrationRecord {
    pub workbench_id: String,
    pub script_name: String,
    pub checksum: String,
    pub applied_at: String,
}

pub struct MigrationDispatcher;

impl MigrationDispatcher {
    /// Computes the deterministic SHA-256 digest of migration SQL content.
    pub fn compute_checksum(sql: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(sql.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Discovers all SQL migration scripts for a given workbench directory.
    pub fn discover_migrations(workbench_dir: &Path) -> Result<Vec<MigrationScript>> {
        let migrations_dir = workbench_dir.join("migrations");
        if !migrations_dir.exists() || !migrations_dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut scripts = Vec::new();
        let mut entries = std::fs::read_dir(&migrations_dir)
            .with_context(|| format!("Failed to read migrations directory: {:?}", migrations_dir))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
            .collect::<Vec<_>>();

        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let sql = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read migration script: {:?}", path))?;
            let checksum = Self::compute_checksum(&sql);
            scripts.push(MigrationScript {
                name,
                path,
                sql,
                checksum,
            });
        }

        Ok(scripts)
    }

    /// Validates that all tables created in the migration script strictly follow
    /// the scoped naming convention `mod_<workbench_id>_*` to prevent kernel pollution.
    pub fn validate_scoped_tables(workbench_id: &str, sql: &str) -> Result<()> {
        let expected_prefix = format!("mod_{}_", workbench_id);
        let re = regex::Regex::new(r"(?i)CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([a-zA-Z0-9_]+)")?;

        for cap in re.captures_iter(sql) {
            let table_name = &cap[1];
            if !table_name.starts_with(&expected_prefix) {
                anyhow::bail!(
                    "Scope violation: Workbench '{workbench_id}' migration attempts to create table '{table_name}'. \
                     Workbench tables must strictly use prefix '{expected_prefix}'."
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_checksum_deterministic() {
        let sql = "CREATE TABLE IF NOT EXISTS mod_quant_test (id TEXT PRIMARY KEY);";
        let c1 = MigrationDispatcher::compute_checksum(sql);
        let c2 = MigrationDispatcher::compute_checksum(sql);
        assert_eq!(c1, c2);
        assert_eq!(c1.len(), 64);
    }

    #[test]
    fn test_validate_scoped_tables_accepts_valid_prefix() {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS mod_quant_tearsheets (id TEXT PRIMARY KEY);
            CREATE TABLE mod_quant_orders (id TEXT PRIMARY KEY);
        "#;
        assert!(MigrationDispatcher::validate_scoped_tables("quant", sql).is_ok());
    }

    #[test]
    fn test_validate_scoped_tables_rejects_unprefixed_tables() {
        let sql = "CREATE TABLE IF NOT EXISTS tasks (id TEXT PRIMARY KEY);";
        let res = MigrationDispatcher::validate_scoped_tables("quant", sql);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Scope violation"));
    }
}

