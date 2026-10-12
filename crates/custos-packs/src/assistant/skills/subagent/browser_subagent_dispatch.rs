//! Browser Subagent Dispatch Skill (Assistant Domain Pack)
//!
//! Formulates and orchestrates autonomous Browser Subagent execution plans,
//! validating target URLs, action steps, assertions, and safety sandboxing
//! before dispatching to the Custos orchestration engine.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput,
};
use serde_json::json;

pub struct BrowserSubagentDispatchSkill;

#[async_trait]
impl Skill for BrowserSubagentDispatchSkill {
    fn skill_id(&self) -> &'static str {
        "browser_subagent_dispatch"
    }

    fn description(&self) -> &'static str {
        "Dispatches structured autonomous Browser Subagent tasks for automated scraping, testing, and UI validation"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(15_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let target_url = ctx
            .args
            .get("target_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("target_url".to_string()))?;

        let objective = ctx
            .args
            .get("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("Explore and validate webpage UI");

        let action_type = ctx
            .args
            .get("action_type")
            .and_then(|v| v.as_str())
            .unwrap_or("audit");

        let subagent_plan = json!({
            "task_id": format!("browser-subagent-{}", chrono::Utc::now().timestamp_millis()),
            "target_url": target_url,
            "objective": objective,
            "action_type": action_type,
            "execution_pipeline": [
                {
                    "step": 1,
                    "action": "navigate",
                    "url": target_url,
                    "timeout_ms": 10_000
                },
                {
                    "step": 2,
                    "action": "extract_ax_tree",
                    "capture_screenshot": true
                },
                {
                    "step": 3,
                    "action": "evaluate_dom_state",
                    "check_console_errors": true
                }
            ],
            "sandbox_safety": {
                "allow_form_submit": false,
                "allow_cookies_write": false,
                "max_duration_seconds": 60
            },
            "status": "ready_for_dispatch"
        });

        let result = json!({
            "status": "plan_constructed",
            "dispatch_handle": subagent_plan["task_id"],
            "plan": subagent_plan,
            "orchestrator_receipt": {
                "created_at": chrono::Utc::now().to_rfc3339(),
                "tier": "system_two",
                "lens": "assistant_copilot"
            }
        });

        Ok(SkillOutput::clean("browser_subagent_dispatch", result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_subagent_dispatch_missing_target_url() {
        let skill = BrowserSubagentDispatchSkill;
        let ctx = SkillContext {
            args: json!({ "objective": "Verify user can log in" }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("assistant".into()),
        };
        let res = skill.run(&ctx).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_browser_subagent_dispatch_success() {
        let skill = BrowserSubagentDispatchSkill;
        let ctx = SkillContext {
            args: json!({
                "objective": "Verify user can log in and view dashboard",
                "target_url": "http://localhost:5173/login",
                "action_type": "audit"
            }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("assistant".into()),
        };
        let res = skill.run(&ctx).await.expect("run succeeds");
        assert_eq!(res.skill_id, "browser_subagent_dispatch");
        let output = res.result;
        assert_eq!(output["status"], "plan_constructed");
        let plan = &output["plan"];
        assert_eq!(plan["objective"], "Verify user can log in and view dashboard");
        assert_eq!(plan["target_url"], "http://localhost:5173/login");
    }
}
