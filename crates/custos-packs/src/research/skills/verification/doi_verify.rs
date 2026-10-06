//! DOI Verify Skill
//!
//! Validates DOI existence and fetches bibliographic metadata from CrossRef.
//! Output is always marked `TaintLevel::Untrusted` because it originates
//! from third-party external web services.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};
use serde_json::json;
use std::time::Duration;

pub struct DoiVerifySkill {
    client: reqwest::Client,
}

impl Default for DoiVerifySkill {
    fn default() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("CustosResearchPack/0.1.0 (mailto:research@custos.local)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

impl DoiVerifySkill {
    pub fn new() -> Self {
        Self::default()
    }

    /// Basic DOI format sanity check (e.g. 10.xxxx/yyyy)
    pub fn is_valid_doi_format(doi: &str) -> bool {
        let trimmed = doi.trim();
        trimmed.starts_with("10.") && trimmed.contains('/')
    }
}

#[async_trait]
impl Skill for DoiVerifySkill {
    fn skill_id(&self) -> &'static str {
        "doi_verify"
    }

    fn description(&self) -> &'static str {
        "Verify DOI against CrossRef API and retrieve canonical bibliographic metadata"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::external_read(10_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let raw_doi = ctx
            .args
            .get("doi")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("doi".to_string()))?;

        let doi = raw_doi
            .trim()
            .trim_start_matches("https://doi.org/")
            .trim_start_matches("http://doi.org/");

        if !Self::is_valid_doi_format(doi) {
            return Err(SkillError::InvalidArg(format!(
                "invalid DOI format: '{}'. Expected prefix '10.xxxx/'",
                raw_doi
            )));
        }

        let url = format!("https://api.crossref.org/works/{}", doi);

        let response = match self.client.get(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                return Err(SkillError::FetchError(format!(
                    "CrossRef query failed for {}: {}",
                    doi, e
                )));
            }
        };

        let status = response.status().as_u16();
        let exists = status == 200;

        let mut title = None;
        let mut container = None;

        if exists {
            if let Ok(json_body) = response.json::<serde_json::Value>().await {
                if let Some(msg) = json_body.get("message") {
                    title = msg
                        .get("title")
                        .and_then(|t| t.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    container = msg
                        .get("container-title")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
        }

        let evidence = if exists {
            Some(format!(
                "DOI {} verified on CrossRef: {:?}",
                doi,
                title.as_deref().unwrap_or("Unknown Title")
            ))
        } else {
            None
        };

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "doi": doi,
                "verified": exists,
                "http_status": status,
                "title": title,
                "journal": container,
            }),
            artifacts: vec![],
            evidence_hint: evidence,
            taint: TaintLevel::Untrusted, // External API output is Untrusted
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doi_format_validation() {
        assert!(DoiVerifySkill::is_valid_doi_format(
            "10.1038/s41586-020-2649-2"
        ));
        assert!(DoiVerifySkill::is_valid_doi_format(
            "10.1145/3297858.3304033"
        ));
        assert!(!DoiVerifySkill::is_valid_doi_format("invalid-doi"));
        assert!(!DoiVerifySkill::is_valid_doi_format("https://example.com"));
    }
}
