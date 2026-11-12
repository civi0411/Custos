//! # Custos Components & Extension Host Crate (`custos-packs`)
//!
//! Pillar 3: Extension Host Lifecycle for Zero-Footprint Modular Workbenches.

pub mod migrations;
pub mod module;
pub mod registry;
pub mod verifier;

pub use custos_core::contracts::pack::{self, *};
pub use migrations::*;
pub use module::*;
pub use registry::*;
pub use verifier::*;

use custos_runtime::agent::skills::SkillRegistry;
use std::sync::Arc;

/// Constructs a canonical `DomainPackRegistry` loaded with all registered workbench modules.
pub fn create_default_pack_registry() -> DomainPackRegistry {
    let reg = ModuleRegistry::global();
    let ids: Vec<&str> = reg.list().into_iter().map(|m| m.id()).collect();
    reg.create_pack_registry(&ids)
}

/// Resolves the manifest for a known domain pack by its canonical identifier or registered alias.
pub fn get_manifest_for_pack(pack_id: &str) -> Option<DomainPackManifest> {
    ModuleRegistry::global().get_manifest_for_pack(pack_id)
}

/// Resolves a DomainPack instance for a known domain pack by its canonical identifier or alias.
pub fn resolve_pack_by_id(pack_id: &str) -> Option<Arc<dyn DomainPack>> {
    ModuleRegistry::global().resolve_pack_by_id(pack_id)
}

/// Constructs a canonical `SkillRegistry` loaded with cross-pack foundation skills
/// and all registered skills from currently available workbench modules.
pub fn create_default_skill_registry() -> SkillRegistry {
    let reg = ModuleRegistry::global();
    let ids: Vec<&str> = reg.list().into_iter().map(|m| m.id()).collect();
    reg.create_skill_registry(&ids)
}

/// Constructs a `DomainPackRegistry` composed dynamically based on enabled inventory components.
pub fn create_pack_registry_from_inventory(enabled: &[&str]) -> DomainPackRegistry {
    ModuleRegistry::global().create_pack_registry(enabled)
}

/// Constructs a `SkillRegistry` composed dynamically based on enabled inventory components.
pub fn create_skill_registry_from_inventory(enabled: &[&str]) -> SkillRegistry {
    ModuleRegistry::global().create_skill_registry(enabled)
}

/// Dynamic Tool Disclosure: Compiles the exposed tools for the Governed Agent Runtime
/// based on the active Workbench lens and the enabled packs in the registry.
/// Prevents token bloat and hallucination by disclosing only core safe read tools
/// plus domain tools belonging to the active and installed workbench lens.
pub fn compile_governed_tools(
    lens: custos_domain::WorkbenchLens,
    pack_registry: &DomainPackRegistry,
) -> Vec<serde_json::Value> {
    // 1. Cross-lens core safe inspection tools (read-only)
    let mut tools = vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read file contents at specified relative path",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative file path" }
                    },
                    "required": ["path"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "List files and directories in a workspace folder",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory path (defaults to '.')" }
                    }
                }
            }
        }),
    ];

    // 2. Disclose domain tools only if pack for lens is installed/enabled
    if let Some(pack) = pack_registry.get_pack_for_lens(lens) {
        tools.extend(pack.tool_declarations());
    }

    tools
}

/// Registers all verifiers from the domain packs in `pack_registry` into `EvidencePipeline`.
pub fn register_pack_verifiers_into_pipeline(
    pack_registry: &DomainPackRegistry,
    pipeline: &mut custos_core::evidence::EvidencePipeline,
) {
    for verifier in pack_registry.list_all_verifiers() {
        pipeline.register_verifier(Arc::new(verifier::PackVerifierAdapter(verifier)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use custos_domain::session::WorkbenchLens;
    use custos_domain::DomainError;

    struct DummyModule;
    impl WorkbenchModule for DummyModule {
        fn id(&self) -> &'static str {
            "dummy"
        }
        fn aliases(&self) -> &'static [&'static str] {
            &["dummy_alias"]
        }
        fn pack(&self) -> Arc<dyn DomainPack> {
            Arc::new(DummyPack)
        }
        fn register_skills(&self, _registry: &mut dyn custos_core::contracts::skill::SkillSink) {}
    }

    struct DummyPack;
    #[async_trait]
    impl DomainPack for DummyPack {
        fn manifest(&self) -> DomainPackManifest {
            DomainPackManifest {
                id: "dummy".into(),
                name: "Dummy Pack".into(),
                version: "0.1.0".into(),
                lens: WorkbenchLens::Coding,
                description: "Dummy pack".into(),
                supported_jobs: vec![],
                supported_roles: vec![],
                required_capabilities: vec![],
            }
        }
        fn profile(&self) -> crate::pack::PackProfile {
            crate::pack::PackProfile::new(
                "dummy",
                "Dummy Profile",
                custos_domain::oi::ExecutionTopology::T1SingleWorker,
                vec![],
                "dummy_oracle",
                10_000,
                false,
            )
        }
        fn roles(&self) -> Vec<WorkerRole> {
            vec![]
        }
        fn jobs(&self) -> Vec<WorkbenchJob> {
            vec![]
        }
        fn verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
            vec![Arc::new(DummyVerifier)]
        }
        async fn initialize(&self) -> Result<(), DomainError> {
            Ok(())
        }
    }

    struct DummyVerifier;
    #[async_trait]
    impl PackVerifier for DummyVerifier {
        fn verifier_id(&self) -> &'static str {
            "dummy_oracle"
        }
        fn description(&self) -> &'static str {
            "Dummy oracle"
        }
        async fn verify(&self, _ctx: &crate::verifier::VerificationContext) -> crate::verifier::VerificationResult {
            crate::verifier::VerificationResult::Pass {
                detail: "verified".into(),
            }
        }
    }

    #[test]
    fn test_module_registry_registration_and_pack_extraction() {
        let registry = ModuleRegistry::new();
        assert_eq!(registry.list().len(), 0);

        registry.register(Arc::new(DummyModule));
        assert!(registry.has("dummy"));
        assert!(registry.has("dummy_alias"));

        let pack_reg = registry.create_pack_registry(&["dummy_alias"]);
        assert!(pack_reg.has_pack("dummy"));

        let manifest = registry.get_manifest_for_pack("dummy_alias").expect("manifest");
        assert_eq!(manifest.id, "dummy");
        assert_eq!(manifest.lens, WorkbenchLens::Coding);
    }

    #[test]
    fn test_compile_governed_tools_lens_isolation() {
        let registry = DomainPackRegistry::new();
        registry.register(Arc::new(DummyPack));

        // 1. Coding lens reveals base safe tools + edit_file & bash
        let coding_tools = compile_governed_tools(WorkbenchLens::Coding, &registry);
        let coding_tool_names: Vec<&str> = coding_tools
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert!(coding_tool_names.contains(&"read_file"));
        assert!(coding_tool_names.contains(&"list_dir"));

        // 2. Assistant lens discloses base tools
        let assistant_tools = compile_governed_tools(WorkbenchLens::Assistant, &registry);
        let assistant_tool_names: Vec<&str> = assistant_tools
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert!(assistant_tool_names.contains(&"read_file"));
        assert!(assistant_tool_names.contains(&"list_dir"));
    }

    #[tokio::test]
    async fn test_register_pack_verifiers_into_pipeline() {
        use custos_core::evidence::{EvidenceBundle, EvidencePipeline};

        let registry = DomainPackRegistry::new();
        registry.register(Arc::new(DummyPack));
        let mut pipeline = EvidencePipeline::new();
        register_pack_verifiers_into_pipeline(&registry, &mut pipeline);

        let bundle = EvidenceBundle::new(
            "task_test_1",
            "dummy_oracle",
            "Dummy requirement satisfied",
            serde_json::json!({ "status": "ok" }),
        );

        let claim = pipeline.verify_bundle(&bundle).await.expect("verify bundle via pipeline");
        assert!(claim.passed);
        assert_eq!(claim.verifier_id, "dummy_oracle");
    }
}
