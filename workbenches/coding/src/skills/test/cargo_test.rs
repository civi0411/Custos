//! Cargo Test Skill
//!
//! Provides automated execution of `cargo test` within Custos OI constraints.
//! Spawns an isolated child process using `SandboxedProcess` and captures
//! full stdout/stderr and exit codes for deliberation and proof verification.

use async_trait::async_trait;
use custos_engine::agent::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};
use custos_integrations::host::operations::process::SandboxedProcess;
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

        if let Some(ref scope) = ctx.scope_root {
            let canon_root = scope.root.canonicalize().map_err(|e| {
                SkillError::SandboxViolation(format!(
                    "Invalid scope_root '{}': {e}",
                    scope.root.display()
                ))
            })?;
            let canon_ws = std::path::Path::new(workspace)
                .canonicalize()
                .map_err(|e| {
                    SkillError::InvalidArg(format!("Invalid workspace_path '{workspace}': {e}"))
                })?;
            if !canon_ws.starts_with(&canon_root) {
                return Err(SkillError::SandboxViolation(format!(
                    "workspace_path '{}' escapes scope_root '{}'",
                    workspace,
                    canon_root.display()
                )));
            }
        }

        let test_filter = ctx.args.get("filter").and_then(|v| v.as_str());
        if let Some(filter) = test_filter {
            if filter.trim().starts_with('-')
                || filter.split_whitespace().any(|tok| tok.starts_with('-'))
            {
                return Err(SkillError::InvalidArg(format!(
                    "Invalid cargo test filter '{filter}': CLI flags like --list or --skip are forbidden"
                )));
            }
        }

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
            scope_root: None,
        };

        let err = skill.run(&ctx).await.unwrap_err();
        assert!(matches!(err, SkillError::MissingArg(_)));
    }
}
