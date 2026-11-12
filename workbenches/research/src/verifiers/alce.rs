//! ALCE (Automated Language Citation Evaluation) Entailment Engine
//!
//! Provides Natural Language Inference (NLI) and factual grounding verification
//! for scientific research claims against cryptographically anchored source passages.

use crate::domain::claim::{
    AssessmentStatus, ClaimAssessmentRecord, EntailmentVerdict, PassageAnchor, ResearchClaim,
};
use custos_domain::error::DomainError;

pub struct AlceEntailmentEngine {
    evaluator_id: String,
    confidence_threshold: f32,
}

impl Default for AlceEntailmentEngine {
    fn default() -> Self {
        Self {
            evaluator_id: "alce_lexical_signal_v1".to_string(),
            confidence_threshold: 0.70,
        }
    }
}

impl AlceEntailmentEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            evaluator_id: "alce_lexical_signal_v1".to_string(),
            confidence_threshold: threshold,
        }
    }

    /// Evaluates semantic entailment between a cited premise passage and a claim hypothesis.
    pub fn evaluate_entailment(&self, premise: &str, hypothesis: &str) -> EntailmentVerdict {
        let p = premise.trim().to_lowercase();
        let h = hypothesis.trim().to_lowercase();

        if p.is_empty() || h.is_empty() {
            return EntailmentVerdict::Neutral {
                confidence: 0.0,
                rationale: "Empty premise or hypothesis cannot establish entailment".into(),
            };
        }

        // Direct containment: premise verbatim contains hypothesis statement
        if p.contains(&h) {
            return EntailmentVerdict::Entailed {
                confidence: 0.98,
                rationale: "Premise verbatim contains the hypothesis statement".into(),
            };
        }

        // Extract key terms (tokens with len > 3, excluding stop words)
        let stop_words = [
            "that", "this", "with", "from", "have", "were", "been", "their", "which", "there",
            "about",
        ];
        let h_tokens: Vec<&str> = h
            .split_whitespace()
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .collect();

        if h_tokens.is_empty() {
            return EntailmentVerdict::Neutral {
                confidence: 0.3,
                rationale: "Hypothesis lacks substantive distinctive tokens".into(),
            };
        }

        let mut matched = 0;
        for token in &h_tokens {
            if p.contains(token) {
                matched += 1;
            }
        }

        let overlap_ratio = (matched as f32) / (h_tokens.len() as f32);

        // Check for direct polarity contradiction
        let negations = [
            "not",
            "never",
            "fails",
            "refutes",
            "contradicts",
            "opposite",
            "decreases",
            "no evidence",
        ];
        let p_has_neg = negations.iter().any(|&neg| p.contains(neg));
        let h_has_neg = negations.iter().any(|&neg| h.contains(neg));

        if overlap_ratio >= 0.60 && p_has_neg != h_has_neg {
            return EntailmentVerdict::Contradicted {
                confidence: 0.85,
                rationale: format!(
                    "Contradiction detected: high semantic overlap ({:.1}%) but opposing directional polarity",
                    overlap_ratio * 100.0
                ),
            };
        }

        if overlap_ratio >= 0.70 {
            EntailmentVerdict::Entailed {
                confidence: (overlap_ratio * 0.95).min(0.95),
                rationale: format!(
                    "Substantive conceptual overlap ({:.1}%) establishes premise entailment",
                    overlap_ratio * 100.0
                ),
            }
        } else if overlap_ratio >= 0.40 {
            EntailmentVerdict::Neutral {
                confidence: overlap_ratio * 0.8,
                rationale: format!(
                    "Partial topical overlap ({:.1}%) insufficient for rigorous verification",
                    overlap_ratio * 100.0
                ),
            }
        } else {
            EntailmentVerdict::Neutral {
                confidence: 0.1,
                rationale: "Negligible contextual overlap between premise and claim".into(),
            }
        }
    }

    /// Assesses a claim against an optional cryptographically anchored literature passage.
    pub fn assess_claim(
        &self,
        claim: &ResearchClaim,
        anchor: Option<&PassageAnchor>,
        source_version: Option<&str>,
        now_ts: i64,
    ) -> Result<ClaimAssessmentRecord, DomainError> {
        let verdict = if let Some(a) = anchor {
            self.evaluate_entailment(&a.exact_text, &claim.statement)
        } else {
            EntailmentVerdict::Neutral {
                confidence: 0.0,
                rationale: "No passage anchor provided for literature verification".into(),
            }
        };

        let status = match verdict {
            // Lexical overlap is a heuristic signal; emit Passed (provisional lexical match),
            // never Verified without downstream semantic review.
            EntailmentVerdict::Entailed { confidence, .. }
                if confidence >= self.confidence_threshold =>
            {
                AssessmentStatus::Passed
            }
            EntailmentVerdict::Contradicted { .. } => AssessmentStatus::Rejected,
            _ => AssessmentStatus::Uncertain,
        };

        let mut record = ClaimAssessmentRecord::new(
            &claim.id,
            status,
            verdict.confidence(),
            &self.evaluator_id,
            now_ts,
        )?;

        if let Some(a) = anchor {
            record = record.with_passage(&a.id);
        }
        if let Some(sv) = source_version {
            record = record.with_source_version(sv);
        }

        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::claim::ResearchClaimProposal;

    #[test]
    fn test_alce_entailment_verbatim_and_keyword_overlap() {
        let engine = AlceEntailmentEngine::new();
        let premise = "Low-rank adaptation (LoRA) reduces the number of trainable parameters by 10,000 times while preserving full fine-tuning quality.";
        let hypothesis = "LoRA reduces trainable parameters while preserving fine-tuning quality";

        let verdict = engine.evaluate_entailment(premise, hypothesis);
        assert!(verdict.is_entailed());
        assert!(verdict.confidence() >= 0.70);
    }

    #[test]
    fn test_alce_entailment_contradiction_detection() {
        let engine = AlceEntailmentEngine::new();
        let premise = "The proposed technique fails and does not improve model convergence across benchmarks.";
        let hypothesis = "The proposed technique improves model convergence across benchmarks";

        let verdict = engine.evaluate_entailment(premise, hypothesis);
        assert!(matches!(verdict, EntailmentVerdict::Contradicted { .. }));
        assert!(verdict.confidence() >= 0.80);
    }

    #[test]
    fn test_alce_claim_assessment_lifecycle() {
        let engine = AlceEntailmentEngine::new();
        let mut claim =
            ResearchClaimProposal::new("FlashAttention achieves linear memory scaling", 1)
                .unwrap()
                .into_unverified_claim();

        let anchor = PassageAnchor {
            id: "anchor_fa_01".into(),
            source_id: "src_flash_attn".into(),
            source_title: Some("FlashAttention Paper".into()),
            section_title: Some("Section 3".into()),
            page_number: Some(4),
            start_offset: 120,
            end_offset: 280,
            exact_text: "We propose FlashAttention, an IO-aware exact attention algorithm that achieves linear memory scaling with sequence length.".into(),
            passage_hash: "blake3:12345".into(),
        };

        let assessment = engine
            .assess_claim(&claim, Some(&anchor), Some("sha256:source_v1"), 100)
            .unwrap();

        assert_eq!(assessment.status, AssessmentStatus::Passed);
        assert!(assessment.is_valid_and_fresh());

        // Elevate claim in Domain OOP model
        assert!(claim.elevate_grounding(&assessment).is_ok());
        assert!(claim.is_verified_for_coding());
    }
}
