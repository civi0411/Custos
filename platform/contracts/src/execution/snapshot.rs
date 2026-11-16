use serde::{Deserialize, Serialize};

/// Snapshot of computation environment for reproducibility (AEE)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvSnapshot {
    pub python_version: String,
    pub lockfile_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_count: Option<u32>,
    pub hardware: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

impl EnvSnapshot {
    pub fn new(
        python_version: impl Into<String>,
        lockfile_hash: impl Into<String>,
        hardware: impl Into<String>,
    ) -> Self {
        Self {
            python_version: python_version.into(),
            lockfile_hash: lockfile_hash.into(),
            package_count: None,
            hardware: hardware.into(),
            platform: None,
        }
    }
}

