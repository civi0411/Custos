//! AST Search Skill
//!
//! Searches codebase for syntax symbols (functions, structs, traits, enums)
//! and cross-file references.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};
use regex::Regex;
use serde_json::json;
use std::path::{Path, PathBuf};

pub struct AstSearchSkill;

#[async_trait]
impl Skill for AstSearchSkill {
    fn skill_id(&self) -> &'static str {
        "ast_search"
    }

    fn description(&self) -> &'static str {
        "Search codebase for syntax symbols (fn, struct, trait, enum, impl) and call references"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(15_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let symbol = ctx
            .args
            .get("symbol")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("symbol".to_string()))?;

        let workspace_str = ctx
            .args
            .get("workspace_path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let workspace = Path::new(workspace_str);
        if !workspace.exists() {
            return Err(SkillError::InvalidArg(format!(
                "workspace path not found: {}",
                workspace_str
            )));
        }

        let def_pattern = format!(
            r"(pub\s+)?(fn|struct|enum|trait|type|const)\s+{}",
            regex::escape(symbol)
        );
        let def_re = Regex::new(&def_pattern)
            .map_err(|e| SkillError::InvalidArg(format!("regex compilation failed: {}", e)))?;

        let call_pattern = format!(r"\b{}\b", regex::escape(symbol));
        let call_re = Regex::new(&call_pattern)
            .map_err(|e| SkillError::InvalidArg(format!("regex compilation failed: {}", e)))?;

        let mut definitions = Vec::new();
        let mut references = Vec::new();

        let mut stack: Vec<PathBuf> = vec![workspace.to_path_buf()];
        while let Some(dir) = stack.pop() {
            if definitions.len() + references.len() >= 200 {
                break;
            }

            let mut rd = match tokio::fs::read_dir(&dir).await {
                Ok(r) => r,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = rd.next_entry().await {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }

                if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "rs" || ext == "ts" || ext == "js" || ext == "py" || ext == "go" {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            for (line_idx, line) in content.lines().enumerate() {
                                if def_re.is_match(line) {
                                    definitions.push(json!({
                                        "file": path.to_string_lossy(),
                                        "line": line_idx + 1,
                                        "content": line.trim(),
                                    }));
                                } else if call_re.is_match(line) && references.len() < 50 {
                                    references.push(json!({
                                        "file": path.to_string_lossy(),
                                        "line": line_idx + 1,
                                        "content": line.trim(),
                                    }));
                                }
                            }
                        }
                    }
                } else if path.is_dir() {
                    stack.push(path);
                }
            }
        }

        let def_count = definitions.len();
        let ref_count = references.len();

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "symbol": symbol,
                "definitions": definitions,
                "references": references,
                "definitions_count": def_count,
                "references_count": ref_count,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!(
                "ast_search '{}': {} definitions, {} references found",
                symbol, def_count, ref_count
            )),
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ast_search_missing_symbol() {
        let skill = AstSearchSkill;
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
