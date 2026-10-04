//! Cargo Test Skill
//!
//! Provides automated execution of `cargo test` within Custos OI constraints.
//! Spawns an isolated child process using `SandboxedProcess` and captures
//! full stdout/stderr and exit codes for deliberation and proof verification.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::{
    process::SandboxedProcess, Skill, SkillCapabilityProfile, SkillContext, SkillError,
    SkillOutput, TaintLevel,
};
use serde_json::json;

pub struct CargoTestSkill;

#[async_trait]
impl Skill for CargoTestSkill {
    fn skill_id(&self) -> &'static str {
        "cargo_test"
    }

    fn description(&self) -> &'static str {
        "Runs 'cargo test' in the specified workspace or worktree with optional test filter"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile {
            write_set: vec![],
            requires_permit: true, // Process execution requires explicit permit
            taint_output: false,
            max_execution_ms: 120_000,
        }
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let workspace = ctx
            .args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("workspace_path".to_string()))?;

        let test_filter = ctx.args.get("filter").and_then(|v| v.as_str());

        let mut proc = SandboxedProcess::new("cargo")
            .arg("test")
            .arg("--")
            .arg("--nocapture")
            .cwd(workspace)
            .timeout_ms(self.capability_profile().max_execution_ms);

        if let Some(filter) = test_filter {
            proc = proc.arg(filter);
        }

        let output = proc.run().await?;
        let passed = output.succeeded();

        let evidence = if passed {
            Some(format!(
                "cargo_test exit=0: {}",
                output.stdout_tail(200).trim()
            ))
        } else {
            Some(format!(
                "cargo_test failed exit={}: {}",
                output.exit_code,
                output.stderr_tail(300).trim()
            ))
        };

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "exit_code": output.exit_code,
                "passed": passed,
                "stdout_tail": output.stdout_tail(500),
                "stderr_tail": output.stderr_tail(500),
                "duration_ms": output.elapsed_ms,
            }),
            artifacts: vec![],
            evidence_hint: evidence,
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cargo_test_skill_missing_workspace() {
        let skill = CargoTestSkill;
        let ctx = SkillContext {
            args: json!({}),
            task_id: "task_1".into(),
            worker_run_id: "run_1".into(),
            provider_id: "test".into(),
            pack_id: Some("engineering".into()),
        };

        let err = skill.run(&ctx).await.unwrap_err();
        assert!(matches!(err, SkillError::MissingArg(_)));
    }
}
