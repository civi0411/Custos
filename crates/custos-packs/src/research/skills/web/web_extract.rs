//! Web Extract Skill (Research Domain Pack)
//!
//! Fetches public web pages, technical documentation, and academic references,
//! extracting clean structured Markdown while safely sanitizing scripts and styles.
//! Marks output with `TaintLevel::Untrusted` to prevent prompt injection attacks.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput,
};
use regex::Regex;
use serde_json::json;

pub struct WebExtractSkill {
    http_client: reqwest::Client,
}

impl WebExtractSkill {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .user_agent("CustosResearchAgent/1.0 (Custos ADE Research Engine)")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for WebExtractSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Skill for WebExtractSkill {
    fn skill_id(&self) -> &'static str {
        "web_extract"
    }

    fn description(&self) -> &'static str {
        "Fetches external web documentation or articles, extracting clean structured Markdown with citation anchors"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::external_read(45_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let url = ctx
            .args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("url".to_string()))?;

        // Validate scheme (HTTP/HTTPS only)
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(SkillError::InvalidArg(
                "Only http:// and https:// URLs are permitted".to_string(),
            ));
        }

        // Fetch URL content
        let resp = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| SkillError::FetchError(format!("HTTP request failed: {}", e)))?;

        let status = resp.status().as_u16();
        if status >= 400 {
            return Err(SkillError::FetchError(format!(
                "HTTP server returned error status: {}",
                status
            )));
        }

        let html = resp
            .text()
            .await
            .map_err(|e| SkillError::FetchError(format!("Failed to read body: {}", e)))?;

        // Sanitize HTML into clean markdown-like text
        // Strip <script> and <style>
        let script_re = Regex::new(r"(?is)<script.*?>.*?</script>").unwrap();
        let style_re = Regex::new(r"(?is)<style.*?>.*?</style>").unwrap();
        let tag_re = Regex::new(r"<[^>]+>").unwrap();
        let ws_re = Regex::new(r"\n\s*\n+").unwrap();

        let no_script = script_re.replace_all(&html, "");
        let no_style = style_re.replace_all(&no_script, "");
        let text_only = tag_re.replace_all(&no_style, " ");
        let cleaned = ws_re.replace_all(text_only.trim(), "\n\n");

        // Truncate to reasonable context window limit (max 50,000 chars)
        let truncated = if cleaned.len() > 50_000 {
            format!(
                "{}...\n[Truncated for context length]",
                custos_domain::truncate_str_to_char_boundary(&cleaned, 50_000)
            )
        } else {
            cleaned.to_string()
        };

        let result = json!({
            "status": "success",
            "source_url": url,
            "http_status": status,
            "extracted_length": truncated.len(),
            "content_markdown": truncated,
            "provenance": {
                "source": url,
                "taint": "untrusted",
                "extracted_at": chrono::Utc::now().to_rfc3339()
            }
        });

        Ok(SkillOutput::untrusted("web_extract", result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_extract_missing_url() {
        let skill = WebExtractSkill::new();
        let ctx = SkillContext {
            args: json!({}),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("research".into()),
            scope_root: None,
        };
        let res = skill.run(&ctx).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_web_extract_blocks_file_protocol() {
        let skill = WebExtractSkill::new();
        let ctx = SkillContext {
            args: json!({ "url": "file:///etc/passwd" }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("research".into()),
            scope_root: None,
        };
        let res = skill.run(&ctx).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_web_extract_blocks_ftp_protocol() {
        let skill = WebExtractSkill::new();
        let ctx = SkillContext {
            args: json!({ "url": "ftp://example.com/data" }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("research".into()),
            scope_root: None,
        };
        let res = skill.run(&ctx).await;
        assert!(res.is_err());
    }
}
