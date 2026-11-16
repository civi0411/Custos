use serde::{Deserialize, Serialize};

/// Lifecycle state of an Agent Execution Environment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EnvironmentState {
    Initializing,
    Ready,
    Degraded { reason: String },
    SetupFailed { error: String },
    TornDown,
}

impl EnvironmentState {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready | Self::Degraded { .. })
    }

    pub fn can_transition_to(&self, next: &EnvironmentState) -> bool {
        matches!(
            (self, next),
            (Self::Initializing, Self::Ready)
                | (Self::Initializing, Self::Degraded { .. })
                | (Self::Initializing, Self::SetupFailed { .. })
                | (Self::Ready, Self::Degraded { .. })
                | (Self::Ready, Self::TornDown)
                | (Self::Degraded { .. }, Self::Ready)
                | (Self::Degraded { .. }, Self::TornDown)
                | (Self::SetupFailed { .. }, Self::TornDown)
        )
    }
}

