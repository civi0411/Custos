//! Engineering Domain Pack Skills
//!
//! Provides executable skills for AST symbol queries, Git patch operations,
//! and test runner automation within Custos OI constraints.

pub mod ast;
pub mod git;
pub mod test;

pub use ast::AstSearchSkill;
pub use git::GitPatchSkill;
pub use test::CargoTestSkill;

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_runtime::cognitive::skills::SkillRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Register all engineering skills into the central SkillRegistry
pub fn register_engineering_skills(registry: &mut SkillRegistry) {
    registry.register(Arc::new(AstSearchSkill));
    registry.register(Arc::new(GitPatchSkill));
    registry.register(Arc::new(CargoTestSkill));

    // Configure tier permissions for SystemTwo / engineering pack
    registry.set_tier_permissions(
        "system_two",
        "engineering",
        vec![
            "ast_search".into(),
            "git_patch".into(),
            "cargo_test".into(),
            "file_read".into(),
            "lexical_search".into(),
        ],
    );

    // Configure tier permissions for SystemOne / engineering pack (read-only search)
    registry.set_tier_permissions(
        "system_one",
        "engineering",
        vec![
            "ast_search".into(),
            "file_read".into(),
            "lexical_search".into(),
        ],
    );
}

// ---------------------------------------------------------------------------
// Backward-compatibility layer for legacy mock trait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Vec<SkillParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<String>,
}

#[async_trait]
pub trait EngineeringSkill: Send + Sync {
    fn metadata(&self) -> SkillMetadata;
    async fn execute(&self, args: &Value) -> Result<SkillExecutionResult, DomainError>;
}
