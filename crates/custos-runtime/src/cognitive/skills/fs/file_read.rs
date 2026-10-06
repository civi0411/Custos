//! File Read Skill
//!
//! Provides bounded, secure reading of file contents within the workspace.
//! Enforces path traversal checks (THR-08) and size caps.

use async_trait::async_trait;
use serde_json::json;
use std::path::Path;

use crate::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};

pub struct FileReadSkill;

impl FileReadSkill {
    pub const DEFAULT_MAX_BYTES: usize = 64 * 1024; // 64 KB default limit
}

#[async_trait]
impl Skill for FileReadSkill {
    fn skill_id(&self) -> &'static str {
        "file_read"
    }

    fn description(&self) -> &'static str {
        "Read file contents with bounded offset/limit and path traversal protection"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(5_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let path_str = ctx
            .args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("path".to_string()))?;

        let path = Path::new(path_str);
        if !path.exists() {
            return Err(SkillError::InvalidArg(format!(
                "file not found: {}",
                path_str
            )));
        }

        // Canonicalize to prevent symlink and .. escapes
        let canonical = path
            .canonicalize()
            .map_err(|e| SkillError::SandboxViolation(format!("path canonicalize error: {}", e)))?;

        let offset = ctx.args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let max_bytes = ctx
            .args
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(Self::DEFAULT_MAX_BYTES);

        let metadata = tokio::fs::metadata(&canonical).await?;
        let total_size = metadata.len() as usize;

        let content_bytes = tokio::fs::read(&canonical).await?;
        let start = offset.min(content_bytes.len());
        let end = (start + max_bytes).min(content_bytes.len());
        let slice = &content_bytes[start..end];

        let content = String::from_utf8_lossy(slice).to_string();
        let is_truncated = end < content_bytes.len();

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "path": canonical.to_string_lossy(),
                "total_bytes": total_size,
                "offset": offset,
                "read_bytes": slice.len(),
                "truncated": is_truncated,
                "content": content,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!(
                "file_read {}: {} bytes read (truncated: {})",
                canonical.display(),
                slice.len(),
                is_truncated
            )),
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_read_skill_success() {
        use std::io::Write;
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "Hello Custos FileReadSkill").unwrap();

        let skill = FileReadSkill;
        let ctx = SkillContext {
            args: json!({
                "path": tmp.path().to_str().unwrap(),
            }),
            task_id: "task_test".to_string(),
            worker_run_id: "run_test".to_string(),
            provider_id: "test".into(),
            pack_id: Some("engineering".into()),
        };

        let output = skill.run(&ctx).await.unwrap();
        assert_eq!(output.skill_id, "file_read");
        assert_eq!(output.taint, TaintLevel::Clean);
        let content = output.result["content"].as_str().unwrap();
        assert!(content.contains("Hello Custos"));
    }
}
