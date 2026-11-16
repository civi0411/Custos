use std::{fmt, str::FromStr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingEffort {
    Off,
    Low,
    Medium,
    High,
    Max,
}

impl FromStr for ThinkingEffort {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" | "disabled" | "none" => Ok(Self::Off),
            "low" => Ok(Self::Low),
            "medium" | "med" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "max" | "xhigh" => Ok(Self::Max),
            other => Err(format!("unknown thinking effort: '{other}'")),
        }
    }
}

impl fmt::Display for ThinkingEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => write!(f, "off"),
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Max => write!(f, "max"),
        }
    }
}

/// A single selectable effort value advertised by a provider-managed harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingEffortOption {
    pub value: String,
    pub label: String,
}

/// A harness-advertised effort config option mirrored verbatim into goose's
/// `thinking_effort` session option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinkingEffortCapability {
    /// The harness's config option id, e.g. "effort".
    pub option_id: String,
    pub values: Vec<ThinkingEffortOption>,
    pub current: Option<String>,
}

/// How a provider participates in thinking-effort selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThinkingEffortSupport {
    /// The provider doesn't manage effort; callers keep the model-name-based path.
    Unspecified,
    /// The provider manages reasoning itself but has no effort knob for the current model.
    Unsupported,
    /// The provider passes through a harness-advertised effort option.
    Options(ThinkingEffortCapability),
}

