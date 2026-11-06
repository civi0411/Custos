//! Package Repository Implementation
//!
//! Provides durable storage and retrieval of installed package inventory,
//! activation intents, and lifecycle event records.

use crate::connection::DbConnection;
use custos_domain::DomainError;
use rusqlite::OptionalExtension;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInventoryRow {
    pub component_id: String,
    pub version: String,
    pub channel: String,
    pub state: String,
    pub platform: Option<String>,
    pub artifact_digest: Option<String>,
    pub installed_path: Option<String>,
    pub activation_intent: Option<String>,
    pub enabled_in_profile: Option<String>,
    pub installed_at: Option<String>,
    pub updated_at: String,
}

#[derive(Clone)]
pub struct PackageRepository {
    db: DbConnection,
}

impl PackageRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }

    /// Lists all package records in the inventory.
    pub fn list_inventory(&self) -> Result<Vec<PackageInventoryRow>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT component_id, version, channel, state, platform, artifact_digest, \
                 installed_path, activation_intent, enabled_in_profile, installed_at, updated_at \
                 FROM package_inventory ORDER BY component_id ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(PackageInventoryRow {
                    component_id: row.get(0)?,
                    version: row.get(1)?,
                    channel: row.get(2)?,
                    state: row.get(3)?,
                    platform: row.get(4)?,
                    artifact_digest: row.get(5)?,
                    installed_path: row.get(6)?,
                    activation_intent: row.get(7)?,
                    enabled_in_profile: row.get(8)?,
                    installed_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(rows)
    }

    /// Lists component IDs that are currently active ('builtin' or 'installed').
    pub fn list_enabled_components(&self) -> Result<Vec<String>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT component_id FROM package_inventory \
                 WHERE state IN ('builtin', 'installed') \
                   AND (enabled_in_profile IS NULL OR enabled_in_profile != '0') \
                 ORDER BY component_id ASC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let components = stmt
            .query_map([], |row| row.get(0))
            .map_err(|e| DomainError::Validation(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(components)
    }

    /// Records an activation intent before filesystem mutation begins.
    pub fn record_activation_intent(
        &self,
        component_id: &str,
        version: &str,
        platform: &str,
        command_id: &str,
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let intent_json = serde_json::json!({
            "command_id": command_id,
            "requested_at": chrono::Utc::now().to_rfc3339()
        })
        .to_string();

        tx.execute(
            "INSERT INTO package_inventory (component_id, version, platform, state, activation_intent, updated_at) \
             VALUES (?1, ?2, ?3, 'pending_install', ?4, strftime('%Y-%m-%dT%H:%M:%SZ','now')) \
             ON CONFLICT(component_id, version, platform) DO UPDATE SET \
                state = 'pending_install', \
                activation_intent = excluded.activation_intent, \
                updated_at = excluded.updated_at",
            rusqlite::params![component_id, version, platform, intent_json],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        // Also record to append-only event ledger with command idempotency
        let event_id = format!("ev_{}", custos_domain::new_id("act"));
        tx.execute(
            "INSERT INTO package_activation_events (id, component_id, version, event_type, command_id, detail_json) \
             VALUES (?1, ?2, ?3, 'pending_install', ?4, ?5)",
            rusqlite::params![event_id, component_id, version, command_id, intent_json],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    /// Commits an installed package upon successful verification and promotion.
    pub fn commit_installed(
        &self,
        component_id: &str,
        version: &str,
        platform: &str,
        artifact_digest: Option<&str>,
        installed_path: &str,
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.execute(
            "INSERT INTO package_inventory (component_id, version, platform, state, artifact_digest, installed_path, installed_at, updated_at) \
             VALUES (?1, ?2, ?3, 'installed', ?4, ?5, strftime('%Y-%m-%dT%H:%M:%SZ','now'), strftime('%Y-%m-%dT%H:%M:%SZ','now')) \
             ON CONFLICT(component_id, version, platform) DO UPDATE SET \
                state = 'installed', \
                artifact_digest = excluded.artifact_digest, \
                installed_path = excluded.installed_path, \
                activation_intent = NULL, \
                installed_at = excluded.installed_at, \
                updated_at = excluded.updated_at",
            rusqlite::params![component_id, version, platform, artifact_digest, installed_path],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let event_id = format!("ev_{}", custos_domain::new_id("act"));
        tx.execute(
            "INSERT INTO package_activation_events (id, component_id, version, event_type, detail_json) \
             VALUES (?1, ?2, ?3, 'installed', ?4)",
            rusqlite::params![event_id, component_id, version, installed_path],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    /// Registers a physically verified local package discovery without overwriting disabled or custom states.
    pub fn register_discovered_candidate(
        &self,
        component_id: &str,
        version: &str,
        platform: &str,
        artifact_digest: &str,
        installed_path: &str,
    ) -> Result<bool, DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let existing_state: Option<String> = tx
            .query_row(
                "SELECT state FROM package_inventory WHERE component_id = ?1 AND version = ?2 AND platform = ?3",
                rusqlite::params![component_id, version, platform],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        if let Some(state) = existing_state {
            if state == "disabled" {
                // Do not override user's explicit disable intent at restart
                return Ok(false);
            }
            tx.execute(
                "UPDATE package_inventory SET artifact_digest = ?1, installed_path = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
                 WHERE component_id = ?3 AND version = ?4 AND platform = ?5",
                rusqlite::params![artifact_digest, installed_path, component_id, version, platform],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        } else {
            tx.execute(
                "INSERT INTO package_inventory (component_id, version, platform, state, artifact_digest, installed_path, installed_at, updated_at) \
                 VALUES (?1, ?2, ?3, 'installed', ?4, ?5, strftime('%Y-%m-%dT%H:%M:%SZ','now'), strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
                rusqlite::params![component_id, version, platform, artifact_digest, installed_path],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

            let event_id = format!("ev_{}", custos_domain::new_id("act"));
            tx.execute(
                "INSERT INTO package_activation_events (id, component_id, version, event_type, detail_json) \
                 VALUES (?1, ?2, ?3, 'discovered', ?4)",
                rusqlite::params![event_id, component_id, version, installed_path],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(true)
    }

    /// Marks a package installation as failed.
    pub fn mark_failed(
        &self,
        component_id: &str,
        version: &str,
        platform: &str,
        reason: &str,
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.execute(
            "UPDATE package_inventory SET \
                state = 'failed', \
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
             WHERE component_id = ?1 AND version = ?2 AND platform = ?3",
            rusqlite::params![component_id, version, platform],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let event_id = format!("ev_{}", custos_domain::new_id("act"));
        tx.execute(
            "INSERT INTO package_activation_events (id, component_id, version, event_type, detail_json) \
             VALUES (?1, ?2, ?3, 'failed', ?4)",
            rusqlite::params![event_id, component_id, version, reason],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    /// Enables an installed package for the active profile.
    pub fn enable(&self, component_id: &str) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.execute(
            "UPDATE package_inventory SET \
                state = CASE WHEN state = 'disabled' THEN 'installed' ELSE state END, \
                enabled_in_profile = '1', \
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
             WHERE component_id = ?1",
            rusqlite::params![component_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let event_id = format!("ev_{}", custos_domain::new_id("act"));
        tx.execute(
            "INSERT INTO package_activation_events (id, component_id, version, event_type, detail_json) \
             VALUES (?1, ?2, 'active', 'enabled', ?3)",
            rusqlite::params![event_id, component_id, "profile_enabled"],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    /// Disables an installed package from the active profile.
    pub fn disable(&self, component_id: &str) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.execute(
            "UPDATE package_inventory SET \
                state = CASE WHEN state = 'installed' THEN 'disabled' ELSE state END, \
                enabled_in_profile = '0', \
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') \
             WHERE component_id = ?1",
            rusqlite::params![component_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let event_id = format!("ev_{}", custos_domain::new_id("act"));
        tx.execute(
            "INSERT INTO package_activation_events (id, component_id, version, event_type, detail_json) \
             VALUES (?1, ?2, 'active', 'disabled', ?3)",
            rusqlite::params![event_id, component_id, "profile_disabled"],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    /// Seeds the inventory with components from a specific distribution preset.
    pub fn seed_preset(
        &self,
        preset: custos_domain::shared::packages::DistributionPreset,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        for comp in preset.components() {
            conn.execute(
                "INSERT OR IGNORE INTO package_inventory \
                 (component_id, version, channel, state, platform, updated_at) \
                 VALUES (?1, 'builtin', 'stable', 'builtin', 'any', strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
                rusqlite::params![comp],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_inventory_seed_and_lifecycle() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = PackageRepository::new(db);

        // 1. Initial base seed contains only assistant
        let enabled = repo.list_enabled_components().unwrap();
        assert!(enabled.contains(&"assistant".to_string()));
        assert!(!enabled.contains(&"coding".to_string()));
        assert!(!enabled.contains(&"research".to_string()));
        assert!(!enabled.contains(&"quant".to_string()));
        assert!(!enabled.contains(&"custom_bench".to_string()));

        // 2. Seeding Developer preset activates coding
        repo.seed_preset(custos_domain::shared::packages::DistributionPreset::Developer)
            .unwrap();
        let enabled_dev = repo.list_enabled_components().unwrap();
        assert!(enabled_dev.contains(&"assistant".to_string()));
        assert!(enabled_dev.contains(&"coding".to_string()));
        assert!(!enabled_dev.contains(&"research".to_string()));
        assert!(!enabled_dev.contains(&"quant".to_string()));

        // 3. Record activation intent for custom_bench
        repo.record_activation_intent("custom_bench", "1.0.0", "macos-aarch64", "cmd_install_01")
            .unwrap();

        let inventory = repo.list_inventory().unwrap();
        let custom_row = inventory
            .iter()
            .find(|r| r.component_id == "custom_bench")
            .expect("custom_bench row exists");
        assert_eq!(custom_row.state, "pending_install");
        assert!(custom_row.activation_intent.is_some());

        // 3. Commit installed
        repo.commit_installed(
            "custom_bench",
            "1.0.0",
            "macos-aarch64",
            Some("sha256:abc123"),
            "/path/to/custom_bench",
        )
        .unwrap();

        let enabled_after = repo.list_enabled_components().unwrap();
        assert!(enabled_after.contains(&"custom_bench".to_string()));

        let inventory_after = repo.list_inventory().unwrap();
        let active_custom = inventory_after
            .iter()
            .find(|r| r.component_id == "custom_bench")
            .unwrap();
        assert_eq!(active_custom.state, "installed");
        assert_eq!(
            active_custom.installed_path.as_deref(),
            Some("/path/to/custom_bench")
        );
        assert!(active_custom.activation_intent.is_none());
    }

    #[test]
    fn test_local_discovery_auto_probe_commit() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = PackageRepository::new(db);

        // When a package directory already exists locally without any prior user download action,
        // the local discovery probe can directly commit it to inventory.
        repo.commit_installed(
            "quant",
            "0.2.0",
            "universal",
            Some("sha256:local_discovered_digest"),
            "/Users/mac/.custos/packages/quant/0.2.0",
        )
        .unwrap();

        let enabled = repo.list_enabled_components().unwrap();
        assert!(enabled.contains(&"quant".to_string()));

        let inventory = repo.list_inventory().unwrap();
        let quant_row = inventory
            .iter()
            .find(|r| r.component_id == "quant")
            .unwrap();
        assert_eq!(quant_row.state, "installed");
        assert_eq!(quant_row.version, "0.2.0");
        assert_eq!(quant_row.platform.as_deref(), Some("universal"));
        assert_eq!(
            quant_row.installed_path.as_deref(),
            Some("/Users/mac/.custos/packages/quant/0.2.0")
        );
    }

    #[test]
    fn test_discovered_candidate_does_not_overwrite_disabled_pack() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = PackageRepository::new(db);

        // 1. Install quant package
        repo.commit_installed(
            "quant",
            "0.2.0",
            "universal",
            Some("sha256:abc12345"),
            "/Users/mac/.custos/packages/quant/0.2.0",
        )
        .unwrap();

        assert!(repo
            .list_enabled_components()
            .unwrap()
            .contains(&"quant".to_string()));

        // 2. Explicitly disable quant
        repo.disable("quant").unwrap();
        assert!(!repo
            .list_enabled_components()
            .unwrap()
            .contains(&"quant".to_string()));

        // 3. Daemon restart simulates local discovery candidate registration
        let registered = repo
            .register_discovered_candidate(
                "quant",
                "0.2.0",
                "universal",
                "sha256:abc12345",
                "/Users/mac/.custos/packages/quant/0.2.0",
            )
            .unwrap();

        // Must NOT overwrite disabled state
        assert!(!registered);
        assert!(!repo
            .list_enabled_components()
            .unwrap()
            .contains(&"quant".to_string()));

        // 4. Re-enabling restores active admission
        repo.enable("quant").unwrap();
        assert!(repo
            .list_enabled_components()
            .unwrap()
            .contains(&"quant".to_string()));
    }
}
