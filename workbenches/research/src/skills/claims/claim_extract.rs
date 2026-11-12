//! Claim Extract Skill
//!
//! Parses academic texts or paper summaries into atomic factual claims,
//! identifying candidate assertions and their associated citations or DOIs.

use async_trait::async_trait;
use custos_core::contracts::skill::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};
use regex::Regex;
use serde_json::json;

pub struct ClaimExtractSkill;

fn split_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\n' {
            let chunk = text[start..i].trim();
            if !chunk.is_empty() {
                sentences.push(chunk);
            }
            while i < len && (bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == b' ') {
                i += 1;
            }
            start = i;
            continue;
        }

        if (bytes[i] == b'.' || bytes[i] == b'!' || bytes[i] == b'?')
            && i + 1 < len
            && (bytes[i + 1] == b' ' || bytes[i + 1] == b'\t' || bytes[i + 1] == b'\n' || bytes[i + 1] == b'\r')
        {
            let is_doi_or_number = i > 0 && bytes[i - 1].is_ascii_digit() && (i + 2 < len && bytes[i + 2].is_ascii_digit());
            if !is_doi_or_number {
                let chunk = text[start..=i].trim();
                if !chunk.is_empty() {
                    sentences.push(chunk);
                }
                i += 1;
                while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
                    i += 1;
                }
                start = i;
                continue;
            }
        }
        i += 1;
    }

    if start < len {
        let chunk = text[start..].trim();
        if !chunk.is_empty() {
            sentences.push(chunk);
        }
    }

    sentences
}

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

        // Split into sentences safely without relying on regex look-around (which is unsupported in regex crate)
        for (idx, sentence) in split_sentences(text).into_iter().enumerate() {
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
                    "id": format!("claim_{}", idx),
                    "statement": s,
                    "doi": found_doi,
                    "citation_key": found_cite,
                    "level": if found_doi.is_some() || found_cite.is_some() { "l1_heuristic_citation" } else { "l0_ungrounded" },
                    "required": true
                }));
            }
        }

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "extracted_claims_count": claims.len(),
                "claims": claims
            }),
            artifacts: vec![],
            evidence_hint: None,
            taint: TaintLevel::Clean,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_claim_extract_doi_preserved() {
        let skill = ClaimExtractSkill;
        let args = json!({
            "text": "This study demonstrates the mechanism in 10.1145/3292500.3330964 for deep models. Another finding was observed here."
        });
        let ctx = SkillContext::new(
            args,
            "test_task",
            "test_run",
            "test_provider",
            Some("research".to_string()),
        );

        let output = skill.run(&ctx).await.expect("execute skill");
        let claims = output.result["claims"].as_array().expect("claims array");
        assert_eq!(claims.len(), 2);
        assert_eq!(
            claims[0]["doi"].as_str(),
            Some("10.1145/3292500.3330964")
        );
        assert_eq!(claims[0]["level"].as_str(), Some("l1_heuristic_citation"));
    }
}
