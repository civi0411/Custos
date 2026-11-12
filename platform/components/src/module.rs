//! Custos Workbench Extension SPI & Module Lifecycle Interface
//!
//! Provides the canonical contract for domain workbenches to dynamically register
//! their packs, skills, and optional schema migrations without coupling into platform core.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

pub use custos_core::contracts::pack::WorkbenchModule;
use custos_adapters::host::operations::{FetchUrlSkill, FileReadSkill, LexicalSearchSkill};
use custos_core::contracts::pack::{DomainPack, DomainPackManifest};
use custos_runtime::agent::skills::SkillRegistry;

use crate::registry::DomainPackRegistry;

/// Central registry housing active WorkbenchModule implementations.
pub struct ModuleRegistry {
    modules: RwLock<HashMap<String, Arc<dyn WorkbenchModule>>>,
    aliases: RwLock<HashMap<String, String>>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    /// Creates an empty module registry.
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
            aliases: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a workbench module into the registry, mapping canonical id and aliases.
    pub fn register(&self, module: Arc<dyn WorkbenchModule>) {
        let canonical_id = module.id();
        if let Ok(mut lock) = self.modules.write() {
            lock.insert(canonical_id.to_string(), module.clone());
        }
        if let Ok(mut alias_lock) = self.aliases.write() {
            for &alias in module.aliases() {
                alias_lock.insert(alias.to_string(), canonical_id.to_string());
            }
        }
    }

    /// Resolves a workbench module by its canonical ID or any registered alias.
    pub fn get(&self, id_or_alias: &str) -> Option<Arc<dyn WorkbenchModule>> {
        if let Ok(lock) = self.modules.read() {
            if let Some(m) = lock.get(id_or_alias) {
                return Some(m.clone());
            }
        }
        let target_id = {
            let alias_lock = self.aliases.read().ok()?;
            alias_lock.get(id_or_alias)?.clone()
        };
        let lock = self.modules.read().ok()?;
        lock.get(&target_id).cloned()
    }

    /// Checks if a workbench module is registered under the given ID or alias.
    pub fn has(&self, id_or_alias: &str) -> bool {
        self.get(id_or_alias).is_some()
    }

    /// Lists all uniquely registered workbench modules.
    pub fn list(&self) -> Vec<Arc<dyn WorkbenchModule>> {
        self.modules
            .read()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Constructs a `DomainPackRegistry` containing packs for all enabled component IDs.
    pub fn create_pack_registry(&self, enabled: &[&str]) -> DomainPackRegistry {
        let registry = DomainPackRegistry::new();
        for id in enabled {
            if let Some(module) = self.get(id) {
                registry.register(module.pack());
            } else {
                tracing::warn!(pack_id = %id, "Workbench module not registered or disabled in current build");
            }
        }
        registry
    }

    /// Constructs a `SkillRegistry` loaded with baseline skills and skills from enabled components.
    pub fn create_skill_registry(&self, enabled: &[&str]) -> SkillRegistry {
        let mut registry = SkillRegistry::new();

        // 1. Cross-pack Shared Foundation Skills
        registry.register(Arc::new(FileReadSkill));
        registry.register(Arc::new(LexicalSearchSkill));
        registry.register(Arc::new(FetchUrlSkill::new()));

        // 2. Register skills for enabled modules
        for id in enabled {
            if let Some(module) = self.get(id) {
                module.register_skills(&mut registry);
            }
        }
        registry
    }

    /// Resolves a DomainPackManifest for a given pack id or alias.
    pub fn get_manifest_for_pack(&self, id_or_alias: &str) -> Option<DomainPackManifest> {
        self.get(id_or_alias).map(|m| m.pack().manifest())
    }

    /// Resolves a DomainPack instance for a given pack id or alias.
    pub fn resolve_pack_by_id(&self, id_or_alias: &str) -> Option<Arc<dyn DomainPack>> {
        self.get(id_or_alias).map(|m| m.pack())
    }

    /// Global shared singleton module registry.
    pub fn global() -> &'static ModuleRegistry {
        static REGISTRY: OnceLock<ModuleRegistry> = OnceLock::new();
        REGISTRY.get_or_init(ModuleRegistry::new)
    }
}
