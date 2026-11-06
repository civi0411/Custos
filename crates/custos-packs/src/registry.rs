//! Central Domain Pack Registry
//!
//! Provides composition-root discovery and lookups for all Domain Packs,
//! Workbench Jobs, Worker Roles (Subagents), and Verifiers.

use custos_domain::session::WorkbenchLens;
use std::collections::HashMap;
use std::sync::Arc;

use crate::pack::{DomainPack, DomainPackManifest, WorkbenchJob, WorkerRole};
use crate::verifier::PackVerifier;

/// Central registry housing all active domain packs in Custos.
pub struct DomainPackRegistry {
    packs: HashMap<String, Arc<dyn DomainPack>>,
}

impl Default for DomainPackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainPackRegistry {
    /// Creates an empty pack registry.
    pub fn new() -> Self {
        Self {
            packs: HashMap::new(),
        }
    }

    /// Registers a domain pack into the registry.
    pub fn register(&mut self, pack: Arc<dyn DomainPack>) {
        let manifest = pack.manifest();
        self.packs.insert(manifest.id.clone(), pack);
    }

    /// Retrieves a domain pack by its unique identifier (e.g. "engineering", "research", "assistant").
    pub fn get_pack(&self, pack_id: &str) -> Option<Arc<dyn DomainPack>> {
        self.packs.get(pack_id).cloned()
    }

    /// Retrieves the domain pack associated with a specific Workbench Lens.
    pub fn get_pack_for_lens(&self, lens: WorkbenchLens) -> Option<Arc<dyn DomainPack>> {
        self.packs
            .values()
            .find(|p| p.manifest().lens == lens)
            .cloned()
    }

    /// Looks up a specialized WorkerRole (Subagent) across all registered packs.
    pub fn get_role(&self, role_id: &str) -> Option<WorkerRole> {
        for pack in self.packs.values() {
            if let Some(role) = pack.roles().into_iter().find(|r| r.role_id == role_id) {
                return Some(role);
            }
        }
        None
    }

    /// Looks up a WorkbenchJob across all registered packs.
    pub fn get_job(&self, job_id: &str) -> Option<WorkbenchJob> {
        for pack in self.packs.values() {
            if let Some(job) = pack.jobs().into_iter().find(|j| j.job_id == job_id) {
                return Some(job);
            }
        }
        None
    }

    /// Looks up a verification oracle across all registered packs by its verifier_id.
    pub fn get_verifier(&self, verifier_id: &str) -> Option<Arc<dyn PackVerifier>> {
        for pack in self.packs.values() {
            if let Some(verifier) = pack
                .verifiers()
                .into_iter()
                .find(|v| v.verifier_id() == verifier_id)
            {
                return Some(verifier);
            }
        }
        None
    }

    /// Unregisters a domain pack by its unique identifier.
    pub fn unregister(&mut self, pack_id: &str) -> Option<Arc<dyn DomainPack>> {
        self.packs.remove(pack_id)
    }

    /// Checks if a domain pack is currently registered.
    pub fn has_pack(&self, pack_id: &str) -> bool {
        self.packs.contains_key(pack_id)
    }

    /// Retrieves the manifest for a specific domain pack if registered.
    pub fn get_pack_manifest(&self, pack_id: &str) -> Option<DomainPackManifest> {
        self.packs.get(pack_id).map(|p| p.manifest())
    }

    /// Lists manifests of all registered domain packs.
    pub fn list_packs(&self) -> Vec<DomainPackManifest> {
        self.packs.values().map(|p| p.manifest()).collect()
    }

    /// Lists all supported Workbench Jobs across all packs.
    pub fn list_all_jobs(&self) -> Vec<WorkbenchJob> {
        self.packs.values().flat_map(|p| p.jobs()).collect()
    }

    /// Lists all supported Worker Roles across all packs.
    pub fn list_all_roles(&self) -> Vec<WorkerRole> {
        self.packs.values().flat_map(|p| p.roles()).collect()
    }

    /// Lists all unique verification oracles across all packs.
    pub fn list_all_verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
        self.packs.values().flat_map(|p| p.verifiers()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quant::QuantPack;

    #[test]
    fn test_dynamic_pack_lifecycle() {
        let mut registry = DomainPackRegistry::new();
        assert_eq!(registry.list_packs().len(), 0);
        assert!(!registry.has_pack("quant"));

        // Register on-demand
        let quant_pack = Arc::new(QuantPack::new());
        registry.register(quant_pack);
        assert!(registry.has_pack("quant"));
        assert_eq!(registry.list_packs().len(), 1);

        let manifest = registry.get_pack_manifest("quant").expect("quant manifest");
        assert_eq!(manifest.id, "quant");
        assert_eq!(manifest.lens, WorkbenchLens::Quant);

        let verifiers = registry.list_all_verifiers();
        assert_eq!(verifiers.len(), 3, "Quant pack must expose 3 verifiers");

        // Unregister on-demand
        let removed = registry.unregister("quant");
        assert!(removed.is_some());
        assert!(!registry.has_pack("quant"));
        assert_eq!(registry.list_packs().len(), 0);
    }
}
