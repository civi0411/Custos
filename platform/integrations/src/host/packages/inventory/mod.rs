//! Pillar 3: Workbench Package Inventory
//!
//! Scans the filesystem for installed workbenches, validates component structure,
//! and tracks installed/enabled package states.

use custos_core::contracts::packages::PackageError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::installer::LocalPackageTransferAdapter;

/// A package detected on the local filesystem during auto-probe discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPackage {
    pub component_id: String,
    pub version: String,
    pub platform: String,
    pub digest: String,
    pub installed_path: PathBuf,
}

/// Scans the given packages directory for locally installed or pre-packaged workbenches.
///
/// Directory layout expected: `<packages_dir>/<component_id>/<version>/...`
/// where a valid version directory contains at least one of `package.bundle`, `custos-worker`, or `manifest.json`.
pub fn scan_local_packages(packages_dir: &Path) -> Result<Vec<DiscoveredPackage>, PackageError> {
    if !packages_dir.exists() || !packages_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut discovered = Vec::new();
    let entries = std::fs::read_dir(packages_dir)?;

    for entry in entries.flatten() {
        let comp_path = entry.path();
        if !comp_path.is_dir() {
            continue;
        }

        let comp_name = match comp_path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if comp_name.starts_with('.')
            || LocalPackageTransferAdapter::sanitize_name(&comp_name).is_err()
        {
            continue;
        }

        let version_entries = match std::fs::read_dir(&comp_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        for v_entry in version_entries.flatten() {
            let v_path = v_entry.path();
            if !v_path.is_dir() {
                continue;
            }

            let version = match v_path.file_name().and_then(|n| n.to_str()) {
                Some(ver) => ver.to_string(),
                None => continue,
            };

            if version.starts_with('.')
                || LocalPackageTransferAdapter::sanitize_name(&version).is_err()
            {
                continue;
            }

            let bundle_path = v_path.join("package.bundle");
            let worker_path = v_path.join("custos-worker");
            let manifest_path = v_path.join("manifest.json");

            let digest_opt = if bundle_path.exists() {
                std::fs::read(&bundle_path).ok().map(|bytes| {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!("sha256:{:x}", hasher.finalize())
                })
            } else if worker_path.exists() {
                std::fs::read(&worker_path).ok().map(|bytes| {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!("sha256:{:x}", hasher.finalize())
                })
            } else if manifest_path.exists() {
                std::fs::read(&manifest_path).ok().map(|bytes| {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!("sha256:{:x}", hasher.finalize())
                })
            } else {
                None
            };

            if let Some(digest) = digest_opt {
                discovered.push(DiscoveredPackage {
                    component_id: comp_name.clone(),
                    version,
                    platform: "universal".to_string(),
                    digest,
                    installed_path: v_path,
                });
            }
        }
    }

    Ok(discovered)
}
