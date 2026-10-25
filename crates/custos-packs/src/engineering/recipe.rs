//! Engineering Diagnose -> Patch -> Verify Recipe
//!
//! Provides a typed pipeline for autonomous engineering workflows with
//! explicit input, intermediate artifacts, failure branches, and verified outcomes.

use custos_domain::repo::SourceSpan;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Input parameters for an engineering diagnose-patch-verify cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineeringRecipeInput {
    pub workspace_root: PathBuf,
    pub issue_description: String,
    pub reproducer_command: Option<String>,
    pub allowed_paths: Vec<String>,
}

/// Output of the diagnostic phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticOutcome {
    pub root_cause: String,
    pub affected_spans: Vec<SourceSpan>,
    pub hypothesis: String,
}

/// Output of the patch generation phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchOutcome {
    pub diff: String,
    pub modified_files: Vec<String>,
    pub patch_digest: String,
}

/// Output of the verification phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOutcome {
    pub passed: bool,
    pub verifier_id: String,
    pub detail: String,
}

/// Failure variants for the engineering recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error_kind", content = "detail")]
pub enum RecipeFailure {
    DiagnoseFailed(String),
    PatchFailed {
        reason: String,
        unparsable_diff: Option<String>,
    },
    ScopeViolation {
        path: String,
        allowed: Vec<String>,
    },
    VerificationFailed {
        verifier_id: String,
        reason: String,
    },
}

impl std::fmt::Display for RecipeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecipeFailure::DiagnoseFailed(reason) => write!(f, "Diagnostic failure: {reason}"),
            RecipeFailure::PatchFailed { reason, .. } => write!(f, "Patch generation failed: {reason}"),
            RecipeFailure::ScopeViolation { path, allowed } => {
                write!(f, "Scope violation: '{path}' is not within allowed paths: {allowed:?}")
            }
            RecipeFailure::VerificationFailed { verifier_id, reason } => {
                write!(f, "Verification failed on {verifier_id}: {reason}")
            }
        }
    }
}

impl std::error::Error for RecipeFailure {}

/// Successful complete outcome of the recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineeringRecipeOutcome {
    pub diagnostic: DiagnosticOutcome,
    pub patch: PatchOutcome,
    pub verification: VerifyOutcome,
}

/// Executor for the diagnose -> patch -> verify pipeline.
pub struct EngineeringRecipePipeline;

impl EngineeringRecipePipeline {
    /// Validates patch paths against the declared allowed scope.
    pub fn validate_scope(
        allowed_paths: &[String],
        modified_files: &[String],
    ) -> Result<(), RecipeFailure> {
        if allowed_paths.is_empty() {
            return Ok(());
        }
        for file in modified_files {
            let allowed = allowed_paths.iter().any(|allowed| {
                file == allowed || file.starts_with(&format!("{allowed}/"))
            });
            if !allowed {
                return Err(RecipeFailure::ScopeViolation {
                    path: file.clone(),
                    allowed: allowed_paths.to_vec(),
                });
            }
        }
        Ok(())
    }

    /// Composes a verified outcome from individual phase results.
    pub fn assemble_outcome(
        diagnostic: DiagnosticOutcome,
        patch: PatchOutcome,
        verification: VerifyOutcome,
        allowed_paths: &[String],
    ) -> Result<EngineeringRecipeOutcome, RecipeFailure> {
        Self::validate_scope(allowed_paths, &patch.modified_files)?;

        if !verification.passed {
            return Err(RecipeFailure::VerificationFailed {
                verifier_id: verification.verifier_id,
                reason: verification.detail,
            });
        }

        Ok(EngineeringRecipeOutcome {
            diagnostic,
            patch,
            verification,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engineering_recipe_assemble_outcome_success() {
        let diagnostic = DiagnosticOutcome {
            root_cause: "Null pointer in worker lease".into(),
            affected_spans: vec![],
            hypothesis: "Check lease before acquire".into(),
        };
        let patch = PatchOutcome {
            diff: "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1 +1 @@\n-old\n+new".into(),
            modified_files: vec!["src/lib.rs".into()],
            patch_digest: "sha256:abc123".into(),
        };
        let verification = VerifyOutcome {
            passed: true,
            verifier_id: "cargo_test_oracle".into(),
            detail: "All 18 tests passed".into(),
        };

        let outcome = EngineeringRecipePipeline::assemble_outcome(
            diagnostic,
            patch,
            verification,
            &["src".into()],
        )
        .expect("Recipe outcome must assemble on valid inputs");

        assert_eq!(outcome.diagnostic.root_cause, "Null pointer in worker lease");
        assert_eq!(outcome.patch.modified_files, vec!["src/lib.rs"]);
        assert!(outcome.verification.passed);
    }

    #[test]
    fn test_engineering_recipe_scope_violation() {
        let diagnostic = DiagnosticOutcome {
            root_cause: "Bug in parser".into(),
            affected_spans: vec![],
            hypothesis: "Edit core".into(),
        };
        let patch = PatchOutcome {
            diff: "diff ...".into(),
            modified_files: vec!["etc/passwd".into()],
            patch_digest: "sha256:bad".into(),
        };
        let verification = VerifyOutcome {
            passed: true,
            verifier_id: "cargo_test_oracle".into(),
            detail: "pass".into(),
        };

        let err = EngineeringRecipePipeline::assemble_outcome(
            diagnostic,
            patch,
            verification,
            &["src".into()],
        )
        .unwrap_err();

        assert!(matches!(err, RecipeFailure::ScopeViolation { .. }));
    }

    #[test]
    fn test_engineering_recipe_verification_failure() {
        let diagnostic = DiagnosticOutcome {
            root_cause: "Off by one".into(),
            affected_spans: vec![],
            hypothesis: "Change limit".into(),
        };
        let patch = PatchOutcome {
            diff: "diff ...".into(),
            modified_files: vec!["src/main.rs".into()],
            patch_digest: "sha256:456".into(),
        };
        let verification = VerifyOutcome {
            passed: false,
            verifier_id: "cargo_test_oracle".into(),
            detail: "Test failed: assertion `left == right` failed".into(),
        };

        let err = EngineeringRecipePipeline::assemble_outcome(
            diagnostic,
            patch,
            verification,
            &["src".into()],
        )
        .unwrap_err();

        assert!(matches!(err, RecipeFailure::VerificationFailed { .. }));
    }
}
