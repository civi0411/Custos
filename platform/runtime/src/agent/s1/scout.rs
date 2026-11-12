//! S1 Fast Workspace Scout (Read-Only Discovery)
//!
//! Rapidly inspects the target workspace to discover project types, manifest files,
//! and complexity metrics. Feeds topology recommendations into the OI compiler.
//!
//! Guarantees:
//! - Strictly empty write-set (write_set: []).
//! - Zero execution permissions or authority requests.

use custos_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Ecosystems and project styles detected by S1 Scout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectEcosystem {
    RustCargo,
    NodePackage,
    Python,
    GoModule,
    Generic,
}

/// S1 Scout discovery report for topology selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutReport {
    pub root_path: PathBuf,
    pub ecosystem: ProjectEcosystem,
    pub is_workspace: bool,
    pub file_count: usize,
    pub detected_manifests: Vec<String>,
    pub recommended_topology: String,
    /// Guaranteed empty write-set
    pub write_set: Vec<String>,
}

/// S1 Scout for zero-risk read-only discovery.
#[derive(Debug, Clone, Default)]
pub struct S1Scout;

impl S1Scout {
    pub fn new() -> Self {
        Self
    }

    /// Returns the write set for this scout, strictly empty by contract.
    pub fn write_set(&self) -> Vec<String> {
        Vec::new()
    }

    /// Fast, shallow inspection of the workspace path.
    pub fn inspect(&self, root: &Path) -> Result<ScoutReport, DomainError> {
        if !root.exists() {
            return Err(DomainError::NotFound {
                kind: "WorkspaceRoot".into(),
                id: root.display().to_string(),
            });
        }

        debug!(path = %root.display(), "S1 Scout inspecting workspace");

        let mut manifests = Vec::new();
        let mut is_workspace = false;
        let mut file_count = 0;

        // Check common manifests in root
        let cargo_toml = root.join("Cargo.toml");
        let package_json = root.join("package.json");
        let pyproject_toml = root.join("pyproject.toml");
        let requirements_txt = root.join("requirements.txt");
        let go_mod = root.join("go.mod");

        let mut ecosystem = ProjectEcosystem::Generic;

        if cargo_toml.exists() {
            manifests.push("Cargo.toml".into());
            ecosystem = ProjectEcosystem::RustCargo;
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    is_workspace = true;
                }
            }
        } else if package_json.exists() {
            manifests.push("package.json".into());
            ecosystem = ProjectEcosystem::NodePackage;
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                if content.contains("\"workspaces\"") {
                    is_workspace = true;
                }
            }
        } else if pyproject_toml.exists() || requirements_txt.exists() {
            if pyproject_toml.exists() {
                manifests.push("pyproject.toml".into());
            }
            if requirements_txt.exists() {
                manifests.push("requirements.txt".into());
            }
            ecosystem = ProjectEcosystem::Python;
        } else if go_mod.exists() {
            manifests.push("go.mod".into());
            ecosystem = ProjectEcosystem::GoModule;
        }

        // Bounded shallow directory walk (limit to 100 entries for fast S1 response)
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten().take(100) {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        file_count += 1;
                    }
                }
            }
        }

        // Determine recommended topology
        let recommended_topology = if is_workspace {
            "T4Worktree".to_string()
        } else if file_count > 50 {
            "T3ReadFanout".to_string()
        } else {
            "T1SingleWorker".to_string()
        };

        Ok(ScoutReport {
            root_path: root.to_path_buf(),
            ecosystem,
            is_workspace,
            file_count,
            detected_manifests: manifests,
            recommended_topology,
            write_set: self.write_set(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_s1_scout_empty_write_set() {
        let scout = S1Scout::new();
        assert!(scout.write_set().is_empty());
    }

    #[test]
    fn test_s1_scout_cargo_workspace() {
        let dir = tempdir().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        std::fs::write(&cargo_path, "[workspace]\nmembers = [\"crate1\"]").unwrap();

        let scout = S1Scout::new();
        let report = scout.inspect(dir.path()).unwrap();

        assert_eq!(report.ecosystem, ProjectEcosystem::RustCargo);
        assert!(report.is_workspace);
        assert_eq!(report.recommended_topology, "T4Worktree");
        assert!(report.write_set.is_empty());
    }

    #[test]
    fn test_s1_scout_simple_project() {
        let dir = tempdir().unwrap();
        let py_path = dir.path().join("requirements.txt");
        std::fs::write(&py_path, "pytest>=7.0").unwrap();

        let scout = S1Scout::new();
        let report = scout.inspect(dir.path()).unwrap();

        assert_eq!(report.ecosystem, ProjectEcosystem::Python);
        assert!(!report.is_workspace);
        assert_eq!(report.recommended_topology, "T1SingleWorker");
        assert!(report.write_set.is_empty());
    }
}
