//! Environment Port Contract (AEE)
//!
//! Governs the lifecycle and verification of physical and logical
//! agent execution environments across all workbench lenses.

use async_trait::async_trait;
use custos_domain::environment::{EnvironmentAssurance, EnvironmentSpec, EnvironmentState};
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Report produced by Environment verification gate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReport {
    pub env_id: String,
    pub passed: bool,
    pub state: EnvironmentState,
    pub assurance: EnvironmentAssurance,
    pub violations: Vec<String>,
}

impl GateReport {
    pub fn ok(
        env_id: impl Into<String>,
        state: EnvironmentState,
        assurance: EnvironmentAssurance,
    ) -> Self {
        Self {
            env_id: env_id.into(),
            passed: true,
            state,
            assurance,
            violations: Vec::new(),
        }
    }

    pub fn failed(
        env_id: impl Into<String>,
        state: EnvironmentState,
        assurance: EnvironmentAssurance,
        violations: Vec<String>,
    ) -> Self {
        Self {
            env_id: env_id.into(),
            passed: false,
            state,
            assurance,
            violations,
        }
    }
}

/// Handle referencing an active provisioned environment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentHandle {
    pub env_id: String,
    pub working_dir: String,
    pub state: EnvironmentState,
    pub assurance: EnvironmentAssurance,
}

#[async_trait]
pub trait EnvironmentPort: Send + Sync {
    /// Allocates and provisions an environment strictly conforming to spec.
    async fn provision(&self, spec: &EnvironmentSpec) -> Result<EnvironmentHandle, DomainError>;

    /// Verifies current environment state and measured invariants against the spec.
    async fn verify(
        &self,
        handle: &EnvironmentHandle,
        spec: &EnvironmentSpec,
    ) -> Result<GateReport, DomainError>;

    /// Idempotently tears down the environment, reclaiming all physical and logical resources.
    async fn teardown(&self, handle: &EnvironmentHandle) -> Result<(), DomainError>;
}

