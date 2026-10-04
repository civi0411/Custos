//! Lexical Search Skill
//!
//! Searches for literal text or regex patterns in files or directories.
//! Returns bounded matching lines with line numbers and snippets.

use async_trait::async_trait;
use regex::Regex;
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};

pub struct LexicalSearchSkill;

impl LexicalSearchSkill {
    pub const DEFAULT_MAX_MATCHES: usize = 100;
}

#[async_trait]
impl Skill for LexicalSearchSkill {
    fn skill_id(&self) -> &'static str {
        "lexical_search"
    }

    fn description(&self) -> &'static str {
        "Search file or directory for string/regex pattern with line numbering"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(10_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let pattern_str = ctx
            .args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("pattern".to_string()))?;

        let path_str = ctx
            .args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("path".to_string()))?;

        let max_matches = ctx
            .args
            .get("max_matches")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(Self::DEFAULT_MAX_MATCHES);

        let is_regex = ctx
            .args
            .get("is_regex")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let target_path = Path::new(path_str);
        if !target_path.exists() {
            return Err(SkillError::InvalidArg(format!(
                "path does not exist: {}",
                path_str
            )));
        }

        let canonical = target_path
            .canonicalize()
            .map_err(|e| SkillError::SandboxViolation(format!("path canonicalize error: {}", e)))?;

        let regex = if is_regex {
            Regex::new(pattern_str)
                .map_err(|e| SkillError::InvalidArg(format!("invalid regex pattern: {}", e)))?
        } else {
            Regex::new(&regex::escape(pattern_str)).map_err(|e| {
                SkillError::InvalidArg(format!("failed to compile escaped pattern: {}", e))
            })?
        };

        let mut matches = Vec::new();

        // If file, scan directly
        if canonical.is_file() {
            Self::scan_file(&canonical, &regex, max_matches, &mut matches).await?;
        } else if canonical.is_dir() {
            Self::scan_dir_iterative(&canonical, &regex, max_matches, &mut matches).await?;
        }

        let match_count = matches.len();

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "path": canonical.to_string_lossy(),
                "pattern": pattern_str,
                "is_regex": is_regex,
                "matches_count": match_count,
                "matches": matches,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!(
                "lexical_search '{}' in {}: {} matches found",
                pattern_str,
                canonical.display(),
                match_count
            )),
            taint: TaintLevel::Clean,
        })
    }
}

impl LexicalSearchSkill {
    async fn scan_file(
        path: &Path,
        re: &Regex,
        max_matches: usize,
        out: &mut Vec<serde_json::Value>,
    ) -> Result<(), SkillError> {
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            for (idx, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    out.push(json!({
                        "file": path.to_string_lossy(),
                        "line_number": idx + 1,
                        "line_content": line.trim_end(),
                    }));
                    if out.len() >= max_matches {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    async fn scan_dir_iterative(
        root_dir: &Path,
        re: &Regex,
        max_matches: usize,
        out: &mut Vec<serde_json::Value>,
    ) -> Result<(), SkillError> {
        let mut stack: Vec<PathBuf> = vec![root_dir.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            if out.len() >= max_matches {
                break;
            }

            let mut read_dir = match tokio::fs::read_dir(&current_dir).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = read_dir.next_entry().await {
                if out.len() >= max_matches {
                    break;
                }
                let path = entry.path();
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();

                // Skip hidden folders and common noise
                if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
                    continue;
                }

                if path.is_file() {
                    Self::scan_file(&path, re, max_matches, out).await?;
                } else if path.is_dir() {
                    stack.push(path);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_lexical_search_skill_file() {
        use std::io::Write;
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "fn authenticate_user() -> bool {{").unwrap();
        writeln!(tmp, "    true").unwrap();
        writeln!(tmp, "}}").unwrap();

        let skill = LexicalSearchSkill;
        let ctx = SkillContext {
            args: json!({
                "path": tmp.path().to_str().unwrap(),
                "pattern": "authenticate_user",
            }),
            task_id: "task_test".to_string(),
            worker_run_id: "run_test".to_string(),
            provider_id: "test".into(),
            pack_id: Some("engineering".into()),
        };

        let output = skill.run(&ctx).await.unwrap();
        assert_eq!(output.skill_id, "lexical_search");
        let matches = output.result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["line_number"], 1);
    }
}
