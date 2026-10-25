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
    pub const MAX_SAFE_READ: usize = 4 * 1024 * 1024; // 4 MB maximum safe read buffer
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
        let scope = ctx.scope_root.as_ref().ok_or(SkillError::NoScope)?;

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

        let canonical_scope = scope
            .root
            .canonicalize()
            .unwrap_or_else(|_| scope.root.clone());

        if !canonical.starts_with(&canonical_scope) {
            return Err(SkillError::OutOfScope(canonical.to_string_lossy().to_string()));
        }

        let offset = ctx.args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let max_bytes = ctx
            .args
            .get("max_bytes")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(Self::DEFAULT_MAX_BYTES);

        // Checked arithmetic: offset.checked_add(max_bytes).ok_or(SkillError::Overflow)?
        let _ = offset.checked_add(max_bytes).ok_or(SkillError::Overflow)?;

        let metadata = tokio::fs::metadata(&canonical).await?;
        let total_size = metadata.len() as usize;

        let read_limit = max_bytes.min(Self::MAX_SAFE_READ);
        let mut file = tokio::fs::File::open(&canonical).await?;
        use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
        file.seek(SeekFrom::Start(offset as u64)).await?;

        let mut buf = vec![0u8; read_limit];
        let bytes_read = file.read(&mut buf).await?;
        buf.truncate(bytes_read);

        let content = String::from_utf8_lossy(&buf).to_string();
        let is_truncated = (offset + bytes_read) < total_size;

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "path": canonical.to_string_lossy(),
                "total_bytes": total_size,
                "offset": offset,
                "read_bytes": bytes_read,
                "truncated": is_truncated,
                "content": content,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!(
                "file_read {}: {} bytes read (truncated: {})",
                canonical.display(),
                bytes_read,
                is_truncated
            )),
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::skills::WorkspaceScope;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_read_skill_success() {
        use std::io::Write;
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "Hello Custos FileReadSkill").unwrap();

        let parent_dir = tmp.path().parent().unwrap().to_path_buf();
        let skill = FileReadSkill;
        let ctx = SkillContext {
            args: json!({
                "path": tmp.path().to_str().unwrap(),
            }),
            task_id: "task_test".to_string(),
            worker_run_id: "run_test".to_string(),
            provider_id: "test".into(),
            pack_id: Some("engineering".into()),
            scope_root: Some(WorkspaceScope::new(parent_dir)),
        };

        let output = skill.run(&ctx).await.unwrap();
        assert_eq!(output.skill_id, "file_read");
        assert_eq!(output.taint, TaintLevel::Clean);
        let content = output.result["content"].as_str().unwrap();
        assert!(content.contains("Hello Custos"));
    }

    #[tokio::test]
    async fn file_read_outside_scope_is_rejected() {
        use std::io::Write;
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "Secret outside scope").unwrap();

        let isolated_dir = std::env::temp_dir().join(format!("custos_scope_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&isolated_dir).unwrap();

        let skill = FileReadSkill;
        let ctx = SkillContext {
            args: json!({
                "path": tmp.path().to_str().unwrap(),
            }),
            task_id: "task_scope".to_string(),
            worker_run_id: "run_scope".to_string(),
            provider_id: "test".into(),
            pack_id: None,
            scope_root: Some(WorkspaceScope::new(isolated_dir.clone())),
        };

        let err = skill.run(&ctx).await.expect_err("Should reject reading file outside scope root");
        assert!(matches!(err, SkillError::OutOfScope(_)));

        let _ = std::fs::remove_dir_all(&isolated_dir);
    }

    #[tokio::test]
    async fn file_read_large_file_does_not_load_fully_into_ram() {
        use std::io::{Seek, SeekFrom, Write};
        let mut tmp = NamedTempFile::new().unwrap();
        // Create a 10 MB sparse file with known content at start and end
        tmp.as_file_mut().write_all(b"START_HEADER_DATA").unwrap();
        tmp.as_file_mut().seek(SeekFrom::Start(10 * 1024 * 1024)).unwrap();
        tmp.as_file_mut().write_all(b"END_FOOTER_DATA").unwrap();

        let parent_dir = tmp.path().parent().unwrap().to_path_buf();
        let skill = FileReadSkill;
        let ctx = SkillContext {
            args: json!({
                "path": tmp.path().to_str().unwrap(),
                "offset": 0,
                "max_bytes": 17,
            }),
            task_id: "task_large".to_string(),
            worker_run_id: "run_large".to_string(),
            provider_id: "test".into(),
            pack_id: None,
            scope_root: Some(WorkspaceScope::new(parent_dir)),
        };

        let output = skill.run(&ctx).await.unwrap();
        assert_eq!(output.result["read_bytes"], 17);
        assert_eq!(output.result["content"], "START_HEADER_DATA");
        assert_eq!(output.result["total_bytes"], 10 * 1024 * 1024 + 15);
        assert_eq!(output.result["truncated"], true);
    }
}
