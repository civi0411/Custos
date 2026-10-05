//! Assistant Domain Pack Skills
//!
//! Provides executable skills for payload locking, contact resolution,
//! and communication safety within Custos OI constraints.

pub mod draft;
pub mod identity;

pub use draft::PayloadLockSkill;
pub use identity::ContactResolveSkill;

use custos_runtime::cognitive::skills::SkillRegistry;
use std::sync::Arc;

/// Register all assistant skills into the central SkillRegistry
pub fn register_assistant_skills(registry: &mut SkillRegistry) {
    registry.register(Arc::new(PayloadLockSkill));
    registry.register(Arc::new(ContactResolveSkill));

    // Configure tier permissions for TierZero / assistant pack
    registry.set_tier_permissions(
        "tier_zero",
        "assistant",
        vec!["contact_resolve".into(), "file_read".into()],
    );

    // Configure tier permissions for SystemOne / assistant pack
    registry.set_tier_permissions(
        "system_one",
        "assistant",
        vec![
            "payload_lock".into(),
            "contact_resolve".into(),
            "file_read".into(),
            "lexical_search".into(),
        ],
    );

    // Configure tier permissions for SystemTwo / assistant pack
    registry.set_tier_permissions(
        "system_two",
        "assistant",
        vec![
            "payload_lock".into(),
            "contact_resolve".into(),
            "file_read".into(),
            "lexical_search".into(),
        ],
    );
}
