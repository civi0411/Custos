use custos_domain::pipeline::DataInvariant;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Coding Workbench Invariant: Guarantees basic syntactic validity and balanced delimiters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstSyntaxInvariant {
    pub path: String,
    pub source_code: String,
}

impl AstSyntaxInvariant {
    pub fn new(path: impl Into<String>, source_code: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            source_code: source_code.into(),
        }
    }
}

impl DataInvariant for AstSyntaxInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.path.trim().is_empty() {
            return Err(DomainError::Validation(
                "Source file path cannot be empty".to_string(),
            ));
        }
        let mut brace = 0i32;
        let mut bracket = 0i32;
        let mut paren = 0i32;

        for ch in self.source_code.chars() {
            match ch {
                '{' => brace += 1,
                '}' => {
                    brace -= 1;
                    if brace < 0 {
                        return Err(DomainError::Validation(format!(
                            "Unmatched closing brace in '{}'",
                            self.path
                        )));
                    }
                }
                '[' => bracket += 1,
                ']' => {
                    bracket -= 1;
                    if bracket < 0 {
                        return Err(DomainError::Validation(format!(
                            "Unmatched closing bracket in '{}'",
                            self.path
                        )));
                    }
                }
                '(' => paren += 1,
                ')' => {
                    paren -= 1;
                    if paren < 0 {
                        return Err(DomainError::Validation(format!(
                            "Unmatched closing parenthesis in '{}'",
                            self.path
                        )));
                    }
                }
                _ => {}
            }
        }

        if brace != 0 || bracket != 0 || paren != 0 {
            return Err(DomainError::Validation(format!(
                "Unbalanced delimiters in '{}' (brace: {}, bracket: {}, paren: {})",
                self.path, brace, bracket, paren
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_syntax_invariant_validation() {
        assert!(AstSyntaxInvariant::new("src/main.rs", "fn main() { println!(\"Hello\"); }")
            .verify()
            .is_ok());
        assert!(AstSyntaxInvariant::new("src/lib.rs", "fn test() { let x = [1, 2; }")
            .verify()
            .is_err());
    }
}

