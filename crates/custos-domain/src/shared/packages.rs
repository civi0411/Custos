//! Pure domain records for workbench components and distribution packages.
//!
//! Enforces Zero-I/O Invariant: Pure data structures, serde serialization,
//! and boundary canonicalization only.

use serde::{Deserialize, Serialize};

/// Canonical workbench component identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComponentId(pub String);

impl ComponentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Validates that a component identifier is well-formed:
    /// - 1..=64 characters
    /// - Lowercase alphanumeric, hyphen, or underscore only
    pub fn validate(raw: &str) -> Result<Self, &'static str> {
        if raw.is_empty() {
            return Err("component_id cannot be empty");
        }
        if raw.len() > 64 {
            return Err("component_id exceeds max length of 64 characters");
        }
        if !raw.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
            return Err("component_id must contain only lowercase alphanumeric, hyphen, or underscore");
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ComponentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ComponentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Operational readiness of an execution engine or worker backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineReadiness {
    /// Engine binary or environment is not present on disk.
    Missing,
    /// Engine is installed but incompatible with current OS, platform, or host version.
    Incompatible(String),
    /// Engine is fully compatible and ready for execution.
    Ready,
}

/// Profile-level enablement status of a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentEnablement {
    Enabled,
    Disabled,
}

/// Resolves legacy aliases to canonical component IDs at boundary layers.
/// Returns None if the identifier is unknown.
pub fn canonical_component_id(raw: &str) -> Option<&'static str> {
    match raw {
        "assistant" | "copilot" => Some("assistant"),
        "coding" | "engineering" => Some("coding"),
        "research" => Some("research"),
        "quant" => Some("quant"),
        _ => None,
    }
}

/// Product distribution presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionPreset {
    Base,       // Assistant only
    Developer,  // Assistant + Coding
    Research,   // Assistant + Research
    Full,       // All four workbenches
}

impl DistributionPreset {
    pub fn components(&self) -> &'static [&'static str] {
        match self {
            Self::Base => &["assistant"],
            Self::Developer => &["assistant", "coding"],
            Self::Research => &["assistant", "research"],
            Self::Full => &["assistant", "coding", "research", "quant"],
        }
    }
}

/// Lifecycle state of an installable package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageState {
    Builtin,
    Installed,
    PendingInstall,
    PendingRemove,
    Failed,
}

impl PackageState {
    /// Whether the package bits exist on disk.
    pub fn is_installed(&self) -> bool {
        matches!(self, Self::Builtin | Self::Installed)
    }

    /// Backwards-compatible alias for is_installed.
    pub fn is_active(&self) -> bool {
        self.is_installed()
    }

    /// Whether this state is terminal (completed installation or failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Builtin | Self::Installed | Self::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_component_id_aliases() {
        assert_eq!(canonical_component_id("copilot"), Some("assistant"));
        assert_eq!(canonical_component_id("assistant"), Some("assistant"));
        assert_eq!(canonical_component_id("engineering"), Some("coding"));
        assert_eq!(canonical_component_id("coding"), Some("coding"));
        assert_eq!(canonical_component_id("research"), Some("research"));
        assert_eq!(canonical_component_id("quant"), Some("quant"));
        assert_eq!(canonical_component_id("unknown_xyz"), None);
    }

    #[test]
    fn test_distribution_presets() {
        assert_eq!(DistributionPreset::Base.components(), &["assistant"]);
        assert_eq!(DistributionPreset::Developer.components(), &["assistant", "coding"]);
        assert_eq!(DistributionPreset::Full.components(), &["assistant", "coding", "research", "quant"]);
    }

    #[test]
    fn test_package_state_active() {
        assert!(PackageState::Builtin.is_active());
        assert!(PackageState::Installed.is_active());
        assert!(!PackageState::PendingInstall.is_active());
        assert!(!PackageState::Failed.is_active());
        assert!(PackageState::Installed.is_installed());
        assert!(PackageState::Failed.is_terminal());
    }

    #[test]
    fn test_component_id_validation() {
        assert!(ComponentId::validate("valid-component_1").is_ok());
        assert!(ComponentId::validate("").is_err());
        assert!(ComponentId::validate("INVALID_CAPS").is_err());
        assert!(ComponentId::validate("with spaces").is_err());
        assert!(ComponentId::validate("a".repeat(65).as_str()).is_err());
    }
}

