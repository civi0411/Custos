//! Capability Manifests, Descriptors & Registry Contracts
//!
//! Declares capabilities provided by tools, sandboxes, runtime engines, or workbench profiles.
//! Under Custos SADE architecture, capabilities declare their availability truthfully:
//! `Available`, `Degraded(reason)`, or `Unavailable(reason)`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub schema_ref: Option<String>,
}

/// Truthful lifecycle status of a capability in the ADE kernel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapabilityStatus {
    Available,
    Degraded { reason: String },
    Unavailable { reason: String },
}

impl CapabilityStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Degraded { reason } | Self::Unavailable { reason } => Some(reason.as_str()),
        }
    }
}

/// Category grouping of ADE capabilities across workbench lenses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityGroup {
    Conversation,
    Code,
    Compute,
    Evidence,
    Browser,
    Personal,
    Coordination,
}

/// Descriptor of a registered capability in the ADE kernel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub title: String,
    pub group: CapabilityGroup,
    pub status: CapabilityStatus,
    pub description: String,
    pub resource_id: Option<String>,
    pub supported_operations: Vec<String>,
    pub version: String,
}

impl CapabilityDescriptor {
    pub fn available(
        id: impl Into<String>,
        title: impl Into<String>,
        group: CapabilityGroup,
        description: impl Into<String>,
        resource_id: Option<&str>,
        supported_operations: Vec<&str>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group,
            status: CapabilityStatus::Available,
            description: description.into(),
            resource_id: resource_id.map(ToString::to_string),
            supported_operations: supported_operations.into_iter().map(ToString::to_string).collect(),
            version: "0.1.0".to_string(),
        }
    }

    pub fn unavailable(
        id: impl Into<String>,
        title: impl Into<String>,
        group: CapabilityGroup,
        description: impl Into<String>,
        resource_id: Option<&str>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group,
            status: CapabilityStatus::Unavailable {
                reason: reason.into(),
            },
            description: description.into(),
            resource_id: resource_id.map(ToString::to_string),
            supported_operations: Vec::new(),
            version: "0.1.0".to_string(),
        }
    }

    pub fn degraded(
        id: impl Into<String>,
        title: impl Into<String>,
        group: CapabilityGroup,
        description: impl Into<String>,
        resource_id: Option<&str>,
        reason: impl Into<String>,
        supported_operations: Vec<&str>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            group,
            status: CapabilityStatus::Degraded {
                reason: reason.into(),
            },
            description: description.into(),
            resource_id: resource_id.map(ToString::to_string),
            supported_operations: supported_operations.into_iter().map(ToString::to_string).collect(),
            version: "0.1.0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_status_predicates() {
        let avail = CapabilityStatus::Available;
        assert!(avail.is_available());
        assert_eq!(avail.reason(), None);

        let degraded = CapabilityStatus::Degraded {
            reason: "Scheduler degraded".to_string(),
        };
        assert!(!degraded.is_available());
        assert_eq!(degraded.reason(), Some("Scheduler degraded"));

        let unavail = CapabilityStatus::Unavailable {
            reason: "PTY host unconfigured".to_string(),
        };
        assert!(!unavail.is_available());
        assert_eq!(unavail.reason(), Some("PTY host unconfigured"));
    }

    #[test]
    fn test_capability_descriptor_serde_roundtrip() {
        let desc = CapabilityDescriptor::available(
            "workspace.git",
            "Workspaces",
            CapabilityGroup::Code,
            "Inspect daemon-owned Git execution workspaces.",
            Some("worktrees"),
            vec!["create", "list", "get", "archive"],
        );

        let json = serde_json::to_string(&desc).expect("serialize");
        let parsed: CapabilityDescriptor = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(desc, parsed);
        assert!(parsed.status.is_available());
        assert_eq!(parsed.supported_operations.len(), 4);
    }

    #[test]
    fn test_capability_unavailable_serde() {
        let desc = CapabilityDescriptor::unavailable(
            "compute.pty",
            "Terminal",
            CapabilityGroup::Compute,
            "Bounded PTY streams.",
            Some("terminal"),
            "Requires local PTY host stream",
        );

        let json = serde_json::to_string(&desc).expect("serialize");
        assert!(json.contains("\"type\":\"unavailable\""));
        assert!(json.contains("Requires local PTY host stream"));
    }
}
