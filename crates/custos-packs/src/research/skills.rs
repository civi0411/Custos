//! Research Domain Pack Skills
//!
//! Provides executable skills for literature search, DOI verification,
//! and factual claim extraction within Custos OI constraints.

pub mod claims;
pub mod verification;

pub use claims::ClaimExtractSkill;
pub use verification::DoiVerifySkill;

use custos_runtime::cognitive::skills::SkillRegistry;
use std::sync::Arc;

/// Register all research skills into the central SkillRegistry
pub fn register_research_skills(registry: &mut SkillRegistry) {
    registry.register(Arc::new(DoiVerifySkill::new()));
    registry.register(Arc::new(ClaimExtractSkill));

    // Configure tier permissions for SystemTwo / research pack
    registry.set_tier_permissions(
        "system_two",
        "research",
        vec![
            "doi_verify".into(),
            "claim_extract".into(),
            "http_fetch".into(),
            "file_read".into(),
            "lexical_search".into(),
        ],
    );

    // Configure tier permissions for SystemOne / research pack
    registry.set_tier_permissions(
        "system_one",
        "research",
        vec![
            "doi_verify".into(),
            "claim_extract".into(),
            "http_fetch".into(),
            "file_read".into(),
        ],
    );
}
