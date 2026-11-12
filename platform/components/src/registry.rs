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
    packs: std::sync::RwLock<HashMap<String, Arc<dyn DomainPack>>>,
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
            packs: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Registers a domain pack into the registry.
    pub fn register(&self, pack: Arc<dyn DomainPack>) {
        let manifest = pack.manifest();
        if let Ok(mut lock) = self.packs.write() {
            lock.insert(manifest.id.clone(), pack);
        }
    }

    /// Retrieves a domain pack by its unique identifier (e.g. "engineering", "research", "assistant").
    pub fn get_pack(&self, pack_id: &str) -> Option<Arc<dyn DomainPack>> {
        self.packs.read().ok()?.get(pack_id).cloned()
    }

    /// Retrieves the domain pack associated with a specific Workbench Lens.
    pub fn get_pack_for_lens(&self, lens: WorkbenchLens) -> Option<Arc<dyn DomainPack>> {
        self.packs
            .read()
            .ok()?
            .values()
            .find(|p| p.manifest().lens == lens)
            .cloned()
    }

    /// Looks up a specialized WorkerRole (Subagent) across all registered packs.
    pub fn get_role(&self, role_id: &str) -> Option<WorkerRole> {
        let packs = self.packs.read().ok()?;
        for pack in packs.values() {
            if let Some(role) = pack.roles().into_iter().find(|r| r.role_id == role_id) {
                return Some(role);
            }
        }
        None
    }

    /// Looks up a WorkbenchJob across all registered packs.
    pub fn get_job(&self, job_id: &str) -> Option<WorkbenchJob> {
        let packs = self.packs.read().ok()?;
        for pack in packs.values() {
            if let Some(job) = pack.jobs().into_iter().find(|j| j.job_id == job_id) {
                return Some(job);
            }
        }
        None
    }

    /// Looks up a verification oracle across all registered packs by its verifier_id.
    pub fn get_verifier(&self, verifier_id: &str) -> Option<Arc<dyn PackVerifier>> {
        let packs = self.packs.read().ok()?;
        for pack in packs.values() {
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
    pub fn unregister(&self, pack_id: &str) -> Option<Arc<dyn DomainPack>> {
        self.packs.write().ok()?.remove(pack_id)
    }

    /// Checks if a domain pack is currently registered.
    pub fn has_pack(&self, pack_id: &str) -> bool {
        self.packs
            .read()
            .map(|p| p.contains_key(pack_id))
            .unwrap_or(false)
    }

    /// Retrieves the manifest for a specific domain pack if registered.
    pub fn get_pack_manifest(&self, pack_id: &str) -> Option<DomainPackManifest> {
        self.packs.read().ok()?.get(pack_id).map(|p| p.manifest())
    }

    /// Lists manifests of all registered domain packs.
    pub fn list_packs(&self) -> Vec<DomainPackManifest> {
        self.packs
            .read()
            .map(|p| p.values().map(|p| p.manifest()).collect())
            .unwrap_or_default()
    }

    /// Collects lazy supervision profiles contributed by currently enabled domain packs.
    pub fn list_supervision_profiles(
        &self,
    ) -> Vec<(String, custos_domain::supervision::SupervisionProfile)> {
        let Ok(packs) = self.packs.read() else {
            return Vec::new();
        };
        packs
            .values()
            .filter_map(|pack| {
                let profile = pack.supervision_profile()?;
                let key = profile
                    .workbench_id
                    .clone()
                    .unwrap_or_else(|| pack.manifest().id);
                Some((key, profile))
            })
            .collect()
    }

    /// Lists all supported Workbench Jobs across all packs.
    pub fn list_all_jobs(&self) -> Vec<WorkbenchJob> {
        self.packs
            .read()
            .map(|p| p.values().flat_map(|p| p.jobs()).collect())
            .unwrap_or_default()
    }

    /// Lists all supported Worker Roles across all packs.
    pub fn list_all_roles(&self) -> Vec<WorkerRole> {
        self.packs
            .read()
            .map(|p| p.values().flat_map(|p| p.roles()).collect())
            .unwrap_or_default()
    }

    /// Lists all unique verification oracles across all packs.
    pub fn list_all_verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
        self.packs
            .read()
            .map(|p| p.values().flat_map(|p| p.verifiers()).collect())
            .unwrap_or_default()
    }

    /// Lists all tool declarations across all registered packs.
    pub fn list_all_tools(&self) -> Vec<serde_json::Value> {
        self.packs
            .read()
            .map(|p| p.values().flat_map(|p| p.tool_declarations()).collect())
            .unwrap_or_default()
    }

    /// Finds the domain pack that declares the given tool name.
    pub fn find_pack_for_tool(&self, tool_name: &str) -> Option<Arc<dyn DomainPack>> {
        let packs = self.packs.read().ok()?;
        for pack in packs.values() {
            for tool in pack.tool_declarations() {
                if tool
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|n| n.as_str())
                    == Some(tool_name)
                {
                    return Some(pack.clone());
                }
            }
        }
        None
    }

    /// Checks if a tool name is declared by any currently registered domain pack.
    pub fn is_tool_registered(&self, tool_name: &str) -> bool {
        self.find_pack_for_tool(tool_name).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_domain::session::WorkbenchLens;
    use custos_domain::DomainError;

    struct TestPack;

    #[async_trait]
    impl DomainPack for TestPack {
        fn manifest(&self) -> DomainPackManifest {
            DomainPackManifest {
                id: "assistant".into(),
                name: "Test Assistant Pack".into(),
                version: "0.1.0".into(),
                lens: WorkbenchLens::Assistant,
                description: "Test pack".into(),
                supported_jobs: vec![],
                supported_roles: vec![],
                required_capabilities: vec![],
            }
        }
        fn profile(&self) -> crate::pack::PackProfile {
            crate::pack::PackProfile::new(
                "assistant",
                "Test Profile",
                custos_domain::oi::ExecutionTopology::T1SingleWorker,
                vec![],
                "test_oracle",
                50_000,
                false,
            )
        }
        fn roles(&self) -> Vec<WorkerRole> {
            Vec::new()
        }
        fn jobs(&self) -> Vec<WorkbenchJob> {
            Vec::new()
        }
        fn verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
            vec![Arc::new(TestVerifier)]
        }
        async fn initialize(&self) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct TestVerifier;

    #[async_trait]
    impl PackVerifier for TestVerifier {
        fn verifier_id(&self) -> &'static str {
            "test_oracle"
        }
        fn description(&self) -> &'static str {
            "Test oracle"
        }
        async fn verify(&self, _ctx: &crate::verifier::VerificationContext) -> crate::verifier::VerificationResult {
            crate::verifier::VerificationResult::Pass { detail: "ok".into() }
        }
    }

    #[test]
    fn test_dynamic_pack_lifecycle() {
        let registry = DomainPackRegistry::new();
        assert_eq!(registry.list_packs().len(), 0);
        assert!(!registry.has_pack("assistant"));

        // Register on-demand
        let assistant_pack = Arc::new(TestPack);
        registry.register(assistant_pack);
        assert!(registry.has_pack("assistant"));
        assert_eq!(registry.list_packs().len(), 1);

        let manifest = registry.get_pack_manifest("assistant").expect("assistant manifest");
        assert_eq!(manifest.id, "assistant");
        assert_eq!(manifest.lens, WorkbenchLens::Assistant);

        let verifiers = registry.list_all_verifiers();
        assert_eq!(verifiers.len(), 1, "Assistant pack exposes 1 verifier");

        // Unregister on-demand
        let removed = registry.unregister("assistant");
        assert!(removed.is_some());
        assert!(!registry.has_pack("assistant"));
        assert_eq!(registry.list_packs().len(), 0);
    }
}
