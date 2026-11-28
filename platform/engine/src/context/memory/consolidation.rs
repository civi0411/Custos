use custos_contracts::{
    ConsolidationPolicy, DomainError, MemoryConsolidationDraft,
};

/// Pure zero-I/O validator enforcing memory consolidation bounds before SQLite commit.
pub struct ConsolidationValidator;

impl ConsolidationValidator {
    pub fn validate(
        draft: &MemoryConsolidationDraft,
        active_fact_count: usize,
        policy: &ConsolidationPolicy,
    ) -> Result<(), DomainError> {
        if draft.new_facts.len() > policy.max_new_facts {
            return Err(DomainError::Validation(format!(
                "Consolidation draft exceeds max_new_facts ({} > {})",
                draft.new_facts.len(),
                policy.max_new_facts
            )));
        }

        for fact in &draft.new_facts {
            if fact.subject.trim().is_empty()
                || fact.predicate.trim().is_empty()
                || fact.object.trim().is_empty()
            {
                return Err(DomainError::Validation(
                    "Consolidated fact triple must be non-empty".into(),
                ));
            }
            if !fact.confidence.is_finite() || !(0.0..=1.0).contains(&fact.confidence) {
                return Err(DomainError::Validation(
                    "Consolidated fact confidence must be finite in [0.0, 1.0]".into(),
                ));
            }
        }

        if active_fact_count > 0 && !draft.archive_fact_ids.is_empty() {
            let max_archive = ((active_fact_count as f32) * policy.max_archive_ratio).ceil() as usize;
            let allowed = max_archive.max(1);
            if draft.archive_fact_ids.len() > allowed {
                return Err(DomainError::Validation(format!(
                    "Consolidation draft archives too many facts ({} > {})",
                    draft.archive_fact_ids.len(),
                    allowed
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_contracts::FactProposal;

    #[test]
    fn validator_enforces_new_fact_and_archive_bounds() {
        let policy = ConsolidationPolicy::default();
        let mut draft = MemoryConsolidationDraft::default();
        for i in 0..7 {
            draft.new_facts.push(FactProposal {
                subject: "s".into(),
                predicate: "p".into(),
                object: format!("o{i}"),
                confidence: 0.9,
                evidence_refs: vec!["cas:1".into()],
            });
        }
        assert!(ConsolidationValidator::validate(&draft, 10, &policy).is_err());

        draft.new_facts.truncate(2);
        draft.archive_fact_ids = vec!["f1".into(), "f2".into(), "f3".into()];
        // 20% of 10 = 2 allowed
        assert!(ConsolidationValidator::validate(&draft, 10, &policy).is_err());

        draft.archive_fact_ids.truncate(2);
        assert!(ConsolidationValidator::validate(&draft, 10, &policy).is_ok());
    }
}

