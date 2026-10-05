//! Payload Lock Skill
//!
//! Computes deterministic cryptographic hashes (SHA-256) of action payloads
//! (recipient, subject, body, attachments) to guarantee Payload Stability
//! Contract (Custos.md §12.4). The resulting digest is locked into the Permit.

use async_trait::async_trait;
use custos_domain::ids::digest;
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};
use serde_json::json;

pub struct PayloadLockSkill;

#[async_trait]
impl Skill for PayloadLockSkill {
    fn skill_id(&self) -> &'static str {
        "payload_lock"
    }

    fn description(&self) -> &'static str {
        "Computes canonical cryptographic SHA-256 digest of action payload for Permit binding"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(3_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let recipient = ctx
            .args
            .get("recipient")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        let subject = ctx
            .args
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        let body = ctx
            .args
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if recipient.is_empty() && subject.is_empty() && body.is_empty() {
            return Err(SkillError::MissingArg(
                "at least one of recipient, subject, or body must be provided".to_string(),
            ));
        }

        // Canonical canonicalization: recipient + "\n" + subject + "\n" + body
        let canonical_bytes = format!("{}\n{}\n{}", recipient, subject, body);
        let payload_digest = digest(canonical_bytes.as_bytes());

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "payload_digest": payload_digest,
                "recipient": recipient,
                "subject": subject,
                "locked": true,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!("payload_lock: digest={}", payload_digest)),
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_payload_lock_deterministic() {
        let skill = PayloadLockSkill;
        let ctx = SkillContext {
            args: json!({
                "recipient": "alice@example.com",
                "subject": "Q4 Sync",
                "body": "Meeting at 3pm."
            }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("assistant".into()),
        };

        let out1 = skill.run(&ctx).await.unwrap();
        let out2 = skill.run(&ctx).await.unwrap();

        assert_eq!(out1.result["payload_digest"], out2.result["payload_digest"]);
        assert_eq!(out1.result["locked"], true);
    }
}
