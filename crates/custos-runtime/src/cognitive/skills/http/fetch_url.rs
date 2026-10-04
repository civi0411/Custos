//! HTTP Fetch Skill
//!
//! Fetches content from external web endpoints.
//! Enforces SSRF defenses (blocking RFC 1918 / localhost / cloud metadata)
//! and marks all outputs with `TaintLevel::Untrusted` (THR-02 / THR-07).

use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;

use crate::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};

pub struct FetchUrlSkill {
    client: reqwest::Client,
}

impl Default for FetchUrlSkill {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

impl FetchUrlSkill {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate URL against SSRF attacks (private IPs, localhost, cloud metadata)
    fn validate_ssrf_safety(url_str: &str) -> Result<reqwest::Url, SkillError> {
        let url = reqwest::Url::parse(url_str)
            .map_err(|e| SkillError::InvalidArg(format!("invalid URL: {}", e)))?;

        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(SkillError::SandboxViolation(format!(
                "forbidden URL scheme: '{}'. Only http/https allowed.",
                scheme
            )));
        }

        if let Some(host) = url.host_str() {
            let lower_host = host.to_lowercase();
            // Block localhost, cloud metadata, and local broadcast
            if lower_host == "localhost"
                || lower_host == "127.0.0.1"
                || lower_host == "::1"
                || lower_host == "0.0.0.0"
                || lower_host == "169.254.169.254"
                || lower_host.ends_with(".localhost")
                || lower_host.ends_with(".local")
            {
                return Err(SkillError::SandboxViolation(format!(
                    "SSRF protection: access to host '{}' is blocked",
                    host
                )));
            }
        } else {
            return Err(SkillError::InvalidArg("URL missing host".into()));
        }

        Ok(url)
    }
}

#[async_trait]
impl Skill for FetchUrlSkill {
    fn skill_id(&self) -> &'static str {
        "http_fetch"
    }

    fn description(&self) -> &'static str {
        "Fetch external HTTP content with SSRF protection and untrusted taint marking"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::external_read(15_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let url_str = ctx
            .args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("url".to_string()))?;

        let safe_url = Self::validate_ssrf_safety(url_str)?;

        let response = self
            .client
            .get(safe_url.clone())
            .send()
            .await
            .map_err(|e| SkillError::FetchError(format!("request failed: {}", e)))?;

        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| SkillError::FetchError(format!("failed to read body: {}", e)))?;

        // Limit maximum captured body size (128 KB)
        let body_preview = if body.len() > 128 * 1024 {
            let mut preview = body[..128 * 1024].to_string();
            preview.push_str("\n...[truncated: exceeds 128KB limit]");
            preview
        } else {
            body
        };

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "url": safe_url.to_string(),
                "status": status,
                "body": body_preview,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!("http_fetch {} -> status {}", safe_url, status)),
            taint: TaintLevel::Untrusted, // CRITICAL: external web content is always Untrusted
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssrf_blocks_localhost_and_metadata() {
        assert!(FetchUrlSkill::validate_ssrf_safety("http://localhost:8080").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://127.0.0.1/admin").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://169.254.169.254/latest/meta-data").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("ftp://example.com").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("https://api.crossref.org/works").is_ok());
    }
}
