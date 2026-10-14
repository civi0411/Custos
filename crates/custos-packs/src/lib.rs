//! Custos Domain Packs
//!
//! Declarative and algorithmic domain packs:
//! - Engineering (skills, code intelligence, workflows across Coding Lens)
//! - Research (literature, extraction, synthesis across Research Lens)
//! - Assistant (personal productivity, memory, agenda across Assistant / Copilot Lens)

pub mod assistant;
pub mod benchmark;
pub mod engineering;
pub mod pack;
pub mod profile;
pub mod registry;
pub mod research;
pub mod verifier;

pub use benchmark::*;
pub use pack::*;
pub use profile::*;
pub use registry::*;
pub use verifier::*;

use custos_runtime::cognitive::skills::{
    FetchUrlSkill, FileReadSkill, LexicalSearchSkill, SkillRegistry,
};
use std::sync::Arc;

/// Constructs a canonical `DomainPackRegistry` loaded with all 3 canonical packs:
/// Engineering, Research, and Assistant.
pub fn create_default_pack_registry() -> DomainPackRegistry {
    let mut registry = DomainPackRegistry::new();
    registry.register(Arc::new(engineering::EngineeringPack::new()));
    registry.register(Arc::new(research::ResearchPack::new()));
    registry.register(Arc::new(assistant::AssistantPack::new()));
    registry
}

/// Constructs a canonical `SkillRegistry` loaded with cross-pack foundation skills
/// and all registered skills from Engineering, Research, and Assistant domain packs.
pub fn create_default_skill_registry() -> SkillRegistry {
    let mut registry = SkillRegistry::new();

    // 1. Cross-pack Shared Foundation Skills
    registry.register(Arc::new(FileReadSkill));
    registry.register(Arc::new(LexicalSearchSkill));
    registry.register(Arc::new(FetchUrlSkill::new()));

    // 2. Domain Pack: Engineering
    engineering::skills::register_engineering_skills(&mut registry);

    // 3. Domain Pack: Research
    research::skills::register_research_skills(&mut registry);

    // 4. Domain Pack: Assistant
    assistant::skills::register_assistant_skills(&mut registry);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::session::WorkbenchLens;

    #[test]
    fn test_create_default_pack_registry_contains_all_packs_jobs_and_roles() {
        let registry = create_default_pack_registry();

        // 1. Pack presence
        assert!(registry.get_pack("engineering").is_some());
        assert!(registry.get_pack("research").is_some());
        assert!(registry.get_pack("assistant").is_some());

        // 2. Lens mapping
        let coding_pack = registry
            .get_pack_for_lens(WorkbenchLens::Coding)
            .expect("coding pack");
        assert_eq!(coding_pack.manifest().id, "engineering");

        let research_pack = registry
            .get_pack_for_lens(WorkbenchLens::Research)
            .expect("research pack");
        assert_eq!(research_pack.manifest().id, "research");

        let assistant_pack = registry
            .get_pack_for_lens(WorkbenchLens::Assistant)
            .expect("assistant pack");
        assert_eq!(assistant_pack.manifest().id, "assistant");

        // 3. Total jobs count = 7 (eng) + 8 (res) + 5 (ast) = 20
        let all_jobs = registry.list_all_jobs();
        assert_eq!(
            all_jobs.len(),
            20,
            "Expected 20 standardized Workbench Jobs"
        );

        // Verify key jobs across all lenses
        assert!(registry.get_job("engineering__bugfix").is_some());
        assert!(registry.get_job("engineering__repo_explain").is_some());
        assert!(registry.get_job("research__deep_research").is_some());
        assert!(registry.get_job("research__source_qa").is_some());
        assert!(registry.get_job("assistant__inbox_triage").is_some());
        assert!(registry.get_job("assistant__mutate_execution").is_some());

        // 4. Total roles count = 4 (eng) + 4 (res) + 4 (ast) = 12
        let all_roles = registry.list_all_roles();
        assert_eq!(all_roles.len(), 12, "Expected 12 specialized Worker Roles");

        // Verify key roles across packs
        let repo_scout = registry.get_role("repo_scout").expect("repo scout role");
        assert!(repo_scout.read_only);
        assert_eq!(repo_scout.pack_id, "engineering");

        let code_implementer = registry
            .get_role("code_implementer")
            .expect("code implementer");
        assert!(!code_implementer.read_only);

        let literature_scout = registry
            .get_role("literature_scout")
            .expect("literature scout");
        assert!(literature_scout.read_only);

        let triage_worker = registry.get_role("triage_worker").expect("triage worker");
        assert!(triage_worker.read_only);

        // 5. Verifiers discovery
        assert!(registry.get_verifier("cargo_test_oracle").is_some());
        assert!(registry.get_verifier("citation_coverage_oracle").is_some());
        assert!(registry.get_verifier("user_acceptance_oracle").is_some());
    }

    #[test]
    fn test_create_default_skill_registry_contains_all_core_skills() {
        let registry = create_default_skill_registry();

        // Foundation
        assert!(registry.contains("file_read"));
        assert!(registry.contains("lexical_search"));
        assert!(registry.contains("http_fetch"));

        // Engineering
        assert!(registry.contains("cargo_test"));
        assert!(registry.contains("git_patch"));
        assert!(registry.contains("ast_search"));
        assert!(registry.contains("component_clone"));
        assert!(registry.contains("local_ui_diagnostic"));

        // Research
        assert!(registry.contains("doi_verify"));
        assert!(registry.contains("claim_extract"));
        assert!(registry.contains("web_extract"));

        // Assistant
        assert!(registry.contains("payload_lock"));
        assert!(registry.contains("contact_resolve"));
        assert!(registry.contains("ax_tree_audit"));
        assert!(registry.contains("browser_subagent_dispatch"));
    }

    #[test]
    fn test_tier_permissions_query_across_packs() {
        let registry = create_default_skill_registry();

        // SystemTwo Engineering
        let eng_s2 = registry
            .permitted_skills_for("system_two", Some("engineering"))
            .expect("engineering s2 perms");
        assert!(eng_s2.contains(&"cargo_test".to_string()));
        assert!(eng_s2.contains(&"git_patch".to_string()));

        // SystemOne Research
        let res_s1 = registry
            .permitted_skills_for("system_one", Some("research"))
            .expect("research s1 perms");
        assert!(res_s1.contains(&"doi_verify".to_string()));
        assert!(!res_s1.contains(&"cargo_test".to_string()));

        // TierZero Assistant
        let ast_t0 = registry
            .permitted_skills_for("tier_zero", Some("assistant"))
            .expect("assistant t0 perms");
        assert!(ast_t0.contains(&"contact_resolve".to_string()));
        assert!(!ast_t0.contains(&"git_patch".to_string()));
    }
}
