use custos_domain::pipeline::DataInvariant;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Research Workbench Invariant: Guarantees claims anchor to immutable source passages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGroundingInvariant {
    pub source_id: String,
    pub document_hash: String,
    pub quote_text: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

impl EvidenceGroundingInvariant {
    pub fn new(
        source_id: impl Into<String>,
        document_hash: impl Into<String>,
        quote_text: impl Into<String>,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            document_hash: document_hash.into(),
            quote_text: quote_text.into(),
            start_offset,
            end_offset,
        }
    }
}

impl DataInvariant for EvidenceGroundingInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.source_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Source ID cannot be empty".to_string(),
            ));
        }
        if self.document_hash.trim().is_empty() {
            return Err(DomainError::Validation(
                "Claim must bind to a valid document content hash".to_string(),
            ));
        }
        if self.quote_text.trim().is_empty() {
            return Err(DomainError::Validation(
                "Claim must cite non-empty grounding passage text".to_string(),
            ));
        }
        if self.start_offset >= self.end_offset {
            return Err(DomainError::Validation(format!(
                "Invalid anchor offsets: start ({}) >= end ({})",
                self.start_offset, self.end_offset
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_grounding_invariant_validation() {
        let valid_claim = EvidenceGroundingInvariant::new(
            "paper_42",
            "sha256_abcd1234",
            "CRISPR-Cas9 exhibits 99.2% target specificity",
            120,
            165,
        );
        assert!(valid_claim.verify().is_ok());
        assert!(EvidenceGroundingInvariant::new("paper_42", "sha", "text", 200, 150)
            .verify()
            .is_err());
        assert!(EvidenceGroundingInvariant::new("paper_42", "sha", "   ", 10, 20)
            .verify()
            .is_err());
    }
}

