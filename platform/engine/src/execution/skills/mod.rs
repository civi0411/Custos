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

pub use async_trait::async_trait;
pub use custos_core::contracts::skill::*;

use std::collections::HashMap;
use std::sync::Arc;

/// Concrete runtime registry where domain packs register their skills.
/// The 9-Router resolves D6 `tool_set` strings through this registry.
#[derive(Default)]
pub struct SkillRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
    tier_permissions: HashMap<(String, String), Vec<String>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        let id = skill.skill_id().to_string();
        tracing::debug!(skill_id = %id, "registered skill");
        self.skills.insert(id, skill);
    }

    pub fn resolve_tool_set(&self, tool_ids: &[String]) -> Vec<Arc<dyn Skill>> {
        tool_ids
            .iter()
            .filter_map(|id| self.skills.get(id).cloned())
            .collect()
    }

    pub fn get(&self, skill_id: &str) -> Option<Arc<dyn Skill>> {
        self.skills.get(skill_id).cloned()
    }

    pub fn contains(&self, skill_id: &str) -> bool {
        self.skills.contains_key(skill_id)
    }

    pub fn all_skill_ids(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }

    pub fn set_tier_permissions(
        &mut self,
        tier: impl Into<String>,
        pack_id: impl Into<String>,
        skill_ids: Vec<String>,
    ) {
        self.tier_permissions
            .insert((tier.into(), pack_id.into()), skill_ids);
    }

    pub fn allowed_skills_for_tier(&self, tier: &str, pack_id: &str) -> Option<&[String]> {
        self.tier_permissions
            .get(&(tier.to_string(), pack_id.to_string()))
            .map(|v| v.as_slice())
    }

    pub fn permitted_skills_for(&self, tier: &str, pack_id: Option<&str>) -> Option<&Vec<String>> {
        let pack = pack_id.unwrap_or("_any");
        self.tier_permissions
            .get(&(tier.to_string(), pack.to_string()))
    }
}

impl SkillSink for SkillRegistry {
    fn register(&mut self, skill: Arc<dyn Skill>) {
        SkillRegistry::register(self, skill);
    }

    fn set_tier_permissions(&mut self, tier: &str, pack_id: &str, skill_ids: Vec<String>) {
        SkillRegistry::set_tier_permissions(self, tier, pack_id, skill_ids);
    }
}

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
