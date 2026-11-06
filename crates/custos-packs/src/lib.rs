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
pub mod quant;
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

/// Constructs a canonical `DomainPackRegistry` loaded with all 4 canonical packs:
/// Engineering, Research, Assistant, and Quant.
pub fn create_default_pack_registry() -> DomainPackRegistry {
    let mut registry = DomainPackRegistry::new();
    registry.register(Arc::new(engineering::EngineeringPack::new()));
    registry.register(Arc::new(research::ResearchPack::new()));
    registry.register(Arc::new(assistant::AssistantPack::new()));
    registry.register(Arc::new(quant::QuantPack::new()));
    registry
}

/// Constructs a canonical `SkillRegistry` loaded with cross-pack foundation skills
/// and all registered skills from Engineering, Research, Assistant, and Quant domain packs.
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

    // 5. Domain Pack: Quant
    quant::skills::register_quant_skills(&mut registry);

    registry
}

/// Constructs a `DomainPackRegistry` composed dynamically based on enabled inventory components.
pub fn create_pack_registry_from_inventory(enabled: &[&str]) -> DomainPackRegistry {
    let mut registry = DomainPackRegistry::new();
    for id in enabled {
        match *id {
            "assistant" | "copilot" => {
                registry.register(Arc::new(assistant::AssistantPack::new()));
            }
            "coding" | "engineering" => {
                registry.register(Arc::new(engineering::EngineeringPack::new()));
            }
            "research" => {
                registry.register(Arc::new(research::ResearchPack::new()));
            }
            "quant" => {
                registry.register(Arc::new(quant::QuantPack::new()));
            }
            other => {
                tracing::warn!(pack_id = %other, "Unknown pack ID in inventory — skipped");
            }
        }
    }
    registry
}

/// Constructs a `SkillRegistry` composed dynamically based on enabled inventory components.
pub fn create_skill_registry_from_inventory(enabled: &[&str]) -> SkillRegistry {
    let mut registry = SkillRegistry::new();

    // Cross-pack Shared Foundation Skills are always available
    registry.register(Arc::new(FileReadSkill));
    registry.register(Arc::new(LexicalSearchSkill));
    registry.register(Arc::new(FetchUrlSkill::new()));

    for id in enabled {
        match *id {
            "coding" | "engineering" => {
                engineering::skills::register_engineering_skills(&mut registry);
            }
            "research" => {
                research::skills::register_research_skills(&mut registry);
            }
            "assistant" | "copilot" => {
                assistant::skills::register_assistant_skills(&mut registry);
            }
            "quant" => {
                quant::skills::register_quant_skills(&mut registry);
            }
            _ => {}
        }
    }
    registry
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
        assert!(registry.get_pack("quant").is_some());

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

        let quant_pack = registry
            .get_pack_for_lens(WorkbenchLens::Quant)
            .expect("quant pack");
        assert_eq!(quant_pack.manifest().id, "quant");

        // 3. Total jobs count = 7 (eng) + 8 (res) + 5 (ast) + 4 (quant) = 24
        let all_jobs = registry.list_all_jobs();
        assert_eq!(
            all_jobs.len(),
            24,
            "Expected 24 standardized Workbench Jobs"
        );

        // Verify key jobs across all lenses
        assert!(registry.get_job("engineering__bugfix").is_some());
        assert!(registry.get_job("engineering__repo_explain").is_some());
        assert!(registry.get_job("research__deep_research").is_some());
        assert!(registry.get_job("research__source_qa").is_some());
        assert!(registry.get_job("assistant__inbox_triage").is_some());
        assert!(registry.get_job("assistant__mutate_execution").is_some());
        assert!(registry.get_job("quant__alpha_discover").is_some());
        assert!(registry.get_job("quant__execution_plan").is_some());

        // 4. Total roles count = 4 (eng) + 4 (res) + 4 (ast) + 4 (quant) = 16
        let all_roles = registry.list_all_roles();
        assert_eq!(all_roles.len(), 16, "Expected 16 specialized Worker Roles");

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

        let alpha_scout = registry.get_role("alpha_scout").expect("alpha scout");
        assert!(alpha_scout.read_only);
        assert_eq!(alpha_scout.pack_id, "quant");

        let backtest_runner = registry
            .get_role("backtest_runner")
            .expect("backtest runner");
        assert!(!backtest_runner.read_only);

        // 5. Verifiers discovery
        assert!(registry.get_verifier("cargo_test_oracle").is_some());
        assert!(registry.get_verifier("citation_coverage_oracle").is_some());
        assert!(registry.get_verifier("user_acceptance_oracle").is_some());
        assert!(registry.get_verifier("deflated_sharpe_verifier").is_some());
        assert!(registry.get_verifier("lookahead_bias_checker").is_some());
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

    #[test]
    fn test_create_pack_registry_from_inventory_assistant_only() {
        let registry = create_pack_registry_from_inventory(&["assistant"]);
        assert!(registry.has_pack("assistant"));
        assert!(!registry.has_pack("engineering"));
        assert!(!registry.has_pack("research"));
        assert!(!registry.has_pack("quant"));

        let skill_reg = create_skill_registry_from_inventory(&["assistant"]);
        assert!(skill_reg.contains("file_read")); // Foundation
        assert!(skill_reg.contains("payload_lock")); // Assistant
        assert!(!skill_reg.contains("cargo_test")); // Engineering not loaded
    }

    #[test]
    fn test_create_pack_registry_from_inventory_developer_preset() {
        let registry = create_pack_registry_from_inventory(&["assistant", "coding"]);
        assert!(registry.has_pack("assistant"));
        assert!(registry.has_pack("engineering"));
        assert!(!registry.has_pack("research"));
        assert!(!registry.has_pack("quant"));
    }

    #[test]
    fn test_compile_governed_tools_lens_isolation() {
        // Developer preset: only assistant and coding are installed
        let registry = create_pack_registry_from_inventory(&["assistant", "coding"]);

        // 1. Coding lens reveals base tools + edit_file & bash
        let coding_tools = compile_governed_tools(WorkbenchLens::Coding, &registry);
        let coding_tool_names: Vec<&str> = coding_tools
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert!(coding_tool_names.contains(&"read_file"));
        assert!(coding_tool_names.contains(&"list_dir"));
        assert!(coding_tool_names.contains(&"edit_file"));
        assert!(coding_tool_names.contains(&"bash"));
        assert!(!coding_tool_names.contains(&"run_backtest")); // Quant not present

        // 2. Quant lens (not installed in developer preset) only discloses base tools
        let quant_tools = compile_governed_tools(WorkbenchLens::Quant, &registry);
        let quant_tool_names: Vec<&str> = quant_tools
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert_eq!(quant_tool_names, vec!["read_file", "list_dir"]);
        assert!(!quant_tool_names.contains(&"run_backtest"));

        // 3. Assistant lens discloses base tools + schedule_task
        let assistant_tools = compile_governed_tools(WorkbenchLens::Assistant, &registry);
        let assistant_tool_names: Vec<&str> = assistant_tools
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap())
            .collect();
        assert!(assistant_tool_names.contains(&"read_file"));
        assert!(assistant_tool_names.contains(&"schedule_task"));
        assert!(!assistant_tool_names.contains(&"edit_file"));
    }
}
