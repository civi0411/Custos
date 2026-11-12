//! Research Domain Pack Skills
//!
//! Provides executable skills for literature search, DOI verification,
//! and factual claim extraction within Custos OI constraints.

pub mod claims;
pub mod verification;
pub mod web;

pub use claims::ClaimExtractSkill;
pub use verification::DoiVerifySkill;
pub use web::WebExtractSkill;

use custos_core::contracts::skill::SkillSink;
use std::sync::Arc;

/// Register all research skills into the central SkillSink
pub fn register_research_skills(registry: &mut dyn SkillSink) {
    registry.register(Arc::new(DoiVerifySkill::new()));
    registry.register(Arc::new(ClaimExtractSkill));
    registry.register(Arc::new(WebExtractSkill::new()));

    // Configure tier permissions for SystemTwo / research pack
    registry.set_tier_permissions(
        "system_two",
        "research",
        vec![
            "doi_verify".into(),
            "claim_extract".into(),
            "web_extract".into(),
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
            "web_extract".into(),
            "http_fetch".into(),
            "file_read".into(),
        ],
    );
}
