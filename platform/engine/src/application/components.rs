//! Component & Pack Lifecycle Application Use Cases

use crate::components::module::ModuleRegistry;
use crate::components::registry::DomainPackRegistry;
use std::sync::Arc;

/// Application handler for component/pack discovery and lifecycle.
pub struct ComponentApplicationService {
    pub pack_registry: Arc<DomainPackRegistry>,
    pub module_registry: Arc<ModuleRegistry>,
}

impl ComponentApplicationService {
    pub fn new(
        pack_registry: Arc<DomainPackRegistry>,
        module_registry: Arc<ModuleRegistry>,
    ) -> Self {
        Self {
            pack_registry,
            module_registry,
        }
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.module_registry
            .list()
            .into_iter()
            .map(|m| m.id().to_string())
            .collect()
    }
}
