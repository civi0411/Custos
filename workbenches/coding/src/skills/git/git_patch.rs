//! Git Patch Skill
//!
//! Applies unified diff patches atomically to an isolated git worktree.
//! Validates patches with `git apply --check` before applying to guarantee
//! atomic application without leaving rejected hunks or corrupted working tree.

use async_trait::async_trait;
use custos_engine::agent::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel, WriteTarget,
};
use custos_integrations::host::operations::process::SandboxedProcess;
use serde_json::json;

pub struct GitPatchSkill;

#[async_trait]
impl Skill for GitPatchSkill {
    fn skill_id(&self) -> &'static str {
        "git_patch"
    }

    fn description(&self) -> &'static str {
        "Apply atomic unified diff patch to git worktree with pre-flight dry-run check"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile {
            write_set: vec![WriteTarget::git_worktree("worktree_patch")],
            requires_permit: true, // Mutating workspace requires permit
            taint_output: false,
            max_execution_ms: 30_000,
        }
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let workspace = ctx
            .args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("workspace_path".to_string()))?;

        let diff = ctx
            .args
            .get("diff")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("diff".to_string()))?;

        if diff.trim().is_empty() {
            return Err(SkillError::InvalidArg("diff cannot be empty".to_string()));
        }

        let ws_path = std::path::Path::new(workspace);
        if let Some(ref scope) = ctx.scope_root {
            let canon_root = scope.root.canonicalize().map_err(|e| {
                SkillError::SandboxViolation(format!(
                    "Invalid scope_root '{}': {e}",
                    scope.root.display()
                ))
            })?;
            let canon_ws = ws_path.canonicalize().map_err(|e| {
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

        // Write diff to a unique temporary file in the workspace
        let patch_name = format!(".custos_pending_{}.patch", custos_contracts::new_id("p"));
        let patch_path = ws_path.join(&patch_name);
        tokio::fs::write(&patch_path, diff.as_bytes()).await?;

        // 1. Dry run: git apply --check <patch_name>
        let dry_run = SandboxedProcess::new("git")
            .arg("apply")
            .arg("--check")
            .arg(&patch_name)
            .cwd(workspace)
            .timeout_ms(10_000)
            .run()
            .await;
        let dry_run = match dry_run {
            Ok(out) => out,
            Err(e) => {
                let _ = tokio::fs::remove_file(&patch_path).await;
                return Err(e);
            }
        };

        if !dry_run.succeeded() {
            // Clean up temporary patch file
            let _ = tokio::fs::remove_file(&patch_path).await;
            return Err(SkillError::SandboxViolation(format!(
                "git apply --check failed: {}",
                dry_run.stderr_tail(500)
            )));
        }

        // 2. Apply patch
        let apply_res = SandboxedProcess::new("git")
            .arg("apply")
            .arg(&patch_name)
            .cwd(workspace)
            .timeout_ms(15_000)
            .run()
            .await;

        // Always clean up temporary patch file
        let _ = tokio::fs::remove_file(&patch_path).await;
        let apply_run = apply_res?;

        if !apply_run.succeeded() {
            return Err(SkillError::SandboxViolation(format!(
                "git apply failed: {}",
                apply_run.stderr_tail(500)
            )));
        }

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "status": "applied",
                "bytes_applied": diff.len(),
                "duration_ms": dry_run.elapsed_ms + apply_run.elapsed_ms,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!(
                "git_patch: {} bytes cleanly applied to {}",
                diff.len(),
                workspace
            )),
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_git_patch_empty_diff() {
        let skill = GitPatchSkill;
        let ctx = SkillContext {
            args: json!({
                "workspace_path": "/tmp",
                "diff": "   "
            }),
            task_id: "task_1".into(),
            worker_run_id: "run_1".into(),
            provider_id: "test".into(),
            pack_id: Some("engineering".into()),
            scope_root: None,
        };

        let err = skill.run(&ctx).await.unwrap_err();
        assert!(matches!(err, SkillError::InvalidArg(_)));
    }
}
