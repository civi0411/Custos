//! Capability Manifests & Permissions
//!
//! Declares capabilities provided by tools, sandboxes, or plugins.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub schema_ref: Option<String>,
}
