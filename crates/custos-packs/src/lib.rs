//! Custos Domain Packs
//!
//! Declarative and algorithmic domain packs:
//! - Engineering (skills, code intelligence, workflows)
//! - Research (literature, extraction, synthesis)
//! - Assistant (personal productivity, memory, agenda)

pub mod assistant;
pub mod benchmark;
pub mod engineering;
pub mod profile;
pub mod research;
pub mod verifier;

pub use benchmark::*;
pub use profile::*;
pub use verifier::*;

use custos_runtime::cognitive::skills::{
    FetchUrlSkill, FileReadSkill, LexicalSearchSkill, SkillRegistry,
};
use std::sync::Arc;

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

        // Research
        assert!(registry.contains("doi_verify"));
        assert!(registry.contains("claim_extract"));

        // Assistant
        assert!(registry.contains("payload_lock"));
        assert!(registry.contains("contact_resolve"));
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
