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
        self.packs.values().find(|p| p.manifest().lens == lens).cloned()
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
            if let Some(verifier) = pack.verifiers().into_iter().find(|v| v.verifier_id() == verifier_id) {
                return Some(verifier);
            }
        }
        None
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
}
