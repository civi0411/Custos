//! Custos Skill Foundation Layer
//!
//! Provides the universal `Skill` trait and `SkillRegistry` that all domain packs use
//! to register their capabilities. The OI Router resolves D6 (`tool_set`) from
//! registered skill IDs at plan-time, enabling auditable, permission-aware tool dispatch.
//!
//! # Architecture
//! ```text
//! RouteDecision.tool_set (D6) ──► SkillRegistry.resolve_tool_set()
//!                                         │
//!                               ┌─────────▼────────────┐
//!                               │  Arc<dyn Skill> list  │
//!                               └─────────┬────────────┘
//!                                         │ run(SkillContext)
//!                                         ▼
//!                                   SkillOutput
//!                                  (artifacts, evidence_hint, taint)
//! ```

pub mod fs;
pub mod http;
pub mod process;
pub mod search;

pub use fs::*;
pub use http::*;
pub use process::*;
pub use search::*;

pub use async_trait::async_trait;
pub use custos_core::contracts::skill::*;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;

    struct EchoSkill;

    #[async_trait]
    impl Skill for EchoSkill {
        fn skill_id(&self) -> &'static str {
            "echo"
        }
        fn description(&self) -> &'static str {
            "Test echo skill"
        }
        fn capability_profile(&self) -> SkillCapabilityProfile {
            SkillCapabilityProfile::read_only(1_000)
        }
        async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
            Ok(SkillOutput::clean(self.skill_id(), ctx.args.clone()))
        }
    }

    #[test]
    fn registry_register_and_resolve() {
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(EchoSkill));
        assert!(registry.contains("echo"));
        let resolved = registry.resolve_tool_set(&["echo".to_string()]);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].skill_id(), "echo");
    }

    #[test]
    fn tier_permissions_set_and_query() {
        let mut registry = SkillRegistry::new();
        registry.set_tier_permissions(
            "system_two",
            "engineering",
            vec!["cargo_test".to_string(), "git_patch".to_string()],
        );
        let perms = registry.permitted_skills_for("system_two", Some("engineering"));
        assert!(perms.is_some());
        assert!(perms.unwrap().contains(&"cargo_test".to_string()));
    }

    #[test]
    fn unknown_skill_ids_are_filtered_gracefully() {
        let registry = SkillRegistry::new();
        let resolved = registry.resolve_tool_set(&["nonexistent".to_string()]);
        assert!(resolved.is_empty());
    }
}
