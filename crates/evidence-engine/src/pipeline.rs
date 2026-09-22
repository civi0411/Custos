//! Evidence Pipeline & Requirement Evaluation
//!
//! Coordinates verifier execution and evaluates if tasks meet all required verification criteria.

use crate::bundle::EvidenceBundle;
use crate::verifier::{
    CitationVerifier, CommandExitCodeVerifier, ExactMatchVerifier, HashVerifier, Verifier,
};
use custos_core_domain::{DomainError, EvidenceRequirement, VerificationClaim};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementEvaluation {
    pub all_satisfied: bool,
    pub total_requirements: usize,
    pub satisfied_count: usize,
    pub unsatisfied_requirements: Vec<String>,
}

pub struct EvidencePipeline {
    verifiers: HashMap<String, Arc<dyn Verifier>>,
}

impl Default for EvidencePipeline {
    fn default() -> Self {
        Self::with_standard_verifiers()
    }
}

impl EvidencePipeline {
    pub fn new() -> Self {
        Self {
            verifiers: HashMap::new(),
        }
    }

    /// Pre-registers standard built-in verifiers (command_exit_code, hash, exact_match, citation).
    pub fn with_standard_verifiers() -> Self {
        let mut pipeline = Self::new();
        pipeline.register_verifier(Arc::new(CommandExitCodeVerifier));
        pipeline.register_verifier(Arc::new(HashVerifier));
        pipeline.register_verifier(Arc::new(ExactMatchVerifier));
        pipeline.register_verifier(Arc::new(CitationVerifier));
        pipeline
    }

    /// Registers a custom verifier.
    pub fn register_verifier(&mut self, verifier: Arc<dyn Verifier>) {
        self.verifiers
            .insert(verifier.verifier_id().to_string(), verifier);
    }

    /// Verifies a single evidence bundle using the appropriate registered verifier.
    pub async fn verify_bundle(
        &self,
        bundle: &EvidenceBundle,
    ) -> Result<VerificationClaim, DomainError> {
        let verifier = self.verifiers.get(&bundle.verifier_type).ok_or_else(|| {
            DomainError::Validation(format!(
                "No verifier registered for type '{}'",
                bundle.verifier_type
            ))
        })?;

        verifier.verify(bundle).await
    }

    /// Evaluates a list of evidence requirements against accumulated verification claims.
    pub fn evaluate_requirements(
        &self,
        requirements: &[EvidenceRequirement],
        claims: &[VerificationClaim],
    ) -> RequirementEvaluation {
        let mut satisfied_count = 0;
        let mut unsatisfied = Vec::new();

        for req in requirements {
            if !req.required {
                satisfied_count += 1;
                continue;
            }

            // Find a passing claim matching requirement kind or description
            let is_satisfied = claims.iter().any(|claim| {
                claim.passed
                    && (claim.claim_statement.contains(&req.description)
                        || claim.verifier_id == req.kind)
            });

            if is_satisfied {
                satisfied_count += 1;
            } else {
                unsatisfied.push(req.description.clone());
            }
        }

        let total = requirements.len();
        RequirementEvaluation {
            all_satisfied: satisfied_count == total,
            total_requirements: total,
            satisfied_count,
            unsatisfied_requirements: unsatisfied,
        }
    }
}
