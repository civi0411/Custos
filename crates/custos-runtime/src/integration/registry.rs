//! Thread-safe Integration Registry
//!
//! Tracks and manages the lifecycle states of external bindings.

use crate::integration::binding::{IntegrationBinding, IntegrationKind, LifecycleState};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Thread-safe registry managing all external integration bindings in Custos
#[derive(Debug, Clone, Default)]
pub struct IntegrationRegistry {
    bindings: Arc<RwLock<HashMap<Uuid, IntegrationBinding>>>,
}

impl IntegrationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new integration binding
    pub async fn register(&self, binding: IntegrationBinding) {
        let mut guard = self.bindings.write().await;
        guard.insert(binding.binding_id, binding);
    }

    /// Get a binding by ID
    pub async fn get(&self, id: &Uuid) -> Option<IntegrationBinding> {
        let guard = self.bindings.read().await;
        guard.get(id).cloned()
    }

    /// Update the lifecycle state of a binding
    pub async fn update_state(&self, id: &Uuid, state: LifecycleState, error: Option<String>) -> Result<()> {
        let mut guard = self.bindings.write().await;
        let entry = guard.get_mut(id).ok_or_else(|| anyhow!("Binding '{}' not found", id))?;
        entry.state = state;
        entry.error_message = error;
        entry.last_health_check = Some(chrono::Utc::now());
        Ok(())
    }

    /// List all bindings of a specific category
    pub async fn list_by_kind(&self, kind: IntegrationKind) -> Vec<IntegrationBinding> {
        let guard = self.bindings.read().await;
        guard
            .values()
            .filter(|b| b.kind == kind)
            .cloned()
            .collect()
    }

    /// List all active and enabled bindings for a category
    pub async fn list_enabled(&self, kind: IntegrationKind) -> Vec<IntegrationBinding> {
        let guard = self.bindings.read().await;
        guard
            .values()
            .filter(|b| b.kind == kind && b.state == LifecycleState::Enabled)
            .cloned()
            .collect()
    }

    /// Total count of registered bindings
    pub async fn count(&self) -> usize {
        let guard = self.bindings.read().await;
        guard.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_binding_lifecycle_transitions() {
        let registry = IntegrationRegistry::new();
        let id = Uuid::new_v4();

        let binding = IntegrationBinding::new(
            id,
            IntegrationKind::ModelConnection,
            "OpenAI Direct",
            "https://api.openai.com/v1",
            "openai/v1",
        );

        registry.register(binding).await;
        assert_eq!(registry.count().await, 1);

        let initial = registry.get(&id).await.unwrap();
        assert_eq!(initial.state, LifecycleState::Configured);

        // Transition: Configured -> Validated
        registry.update_state(&id, LifecycleState::Validated, None).await.unwrap();
        let validated = registry.get(&id).await.unwrap();
        assert_eq!(validated.state, LifecycleState::Validated);

        // Transition: Validated -> Enabled
        registry.update_state(&id, LifecycleState::Enabled, None).await.unwrap();
        let enabled = registry.list_enabled(IntegrationKind::ModelConnection).await;
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "OpenAI Direct");

        // Transition: Enabled -> Degraded on error
        registry
            .update_state(&id, LifecycleState::Degraded, Some("HTTP 429 Rate Limited".into()))
            .await
            .unwrap();
        let degraded = registry.get(&id).await.unwrap();
        assert_eq!(degraded.state, LifecycleState::Degraded);
        assert_eq!(degraded.error_message, Some("HTTP 429 Rate Limited".into()));
    }
}
