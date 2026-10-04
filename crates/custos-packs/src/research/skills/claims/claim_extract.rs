//! Claim Extract Skill
//!
//! Parses academic texts or paper summaries into atomic factual claims,
//! identifying candidate assertions and their associated citations or DOIs.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};
use regex::Regex;
use serde_json::json;

pub struct ClaimExtractSkill;

#[async_trait]
impl Skill for ClaimExtractSkill {
    fn skill_id(&self) -> &'static str {
        "claim_extract"
    }

    fn description(&self) -> &'static str {
        "Extracts atomic factual claims and detected citation references from text"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(5_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let text = ctx
            .args
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("text".to_string()))?;

        let doi_re = Regex::new(r"10\.\d{4,9}/[-._;()/:A-Za-z0-9]+")
            .map_err(|e| SkillError::InvalidArg(format!("regex error: {}", e)))?;
        let cite_bracket_re = Regex::new(r"\[(\d+|[A-Za-z]+(?:\s+et\s+al\.?)?,\s*\d{4})\]")
            .map_err(|e| SkillError::InvalidArg(format!("regex error: {}", e)))?;

        let mut claims = Vec::new();

        // Split into sentences (by . followed by space or newline)
        for (idx, sentence) in text.split('.').enumerate() {
            let s = sentence.trim();
            if s.len() < 15 {
                continue; // Skip very short sentence fragments
            }

            let found_doi = doi_re.find(s).map(|m| m.as_str().to_string());
            let found_cite = cite_bracket_re.find(s).map(|m| m.as_str().to_string());

            let has_assertion_keywords = s.contains("show")
                || s.contains("demonstrat")
                || s.contains("propos")
                || s.contains("observ")
                || s.contains("achiev")
                || s.contains("prov")
                || s.contains("conclud")
                || s.contains("found that");

            if has_assertion_keywords || found_doi.is_some() || found_cite.is_some() {
                claims.push(json!({
                    "id": format!("claim_{}", idx + 1),
                    "statement": s,
                    "doi": found_doi,
                    "citation": found_cite,
                    "verified": found_doi.is_some(),
                }));
            }
        }

        let claim_count = claims.len();

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "claims_count": claim_count,
                "claims": claims,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!("claim_extract: extracted {} claims", claim_count)),
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_claim_extract_success() {
        let skill = ClaimExtractSkill;
        let text = "AlphaFold demonstrates high accuracy in 3D protein structure prediction [Jumper et al., 2021]. We achieved 90% GDT score.";
        let ctx = SkillContext {
            args: json!({ "text": text }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("research".into()),
        };

        let output = skill.run(&ctx).await.unwrap();
        assert_eq!(output.skill_id, "claim_extract");
        let claims = output.result["claims"].as_array().unwrap();
        assert_eq!(claims.len(), 2);
    }
}
