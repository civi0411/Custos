use custos_contracts::DomainError;
use serde::{Deserialize, Serialize};

/// Fencing token identifying the current authoritative writer generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FencingToken {
    pub generation: u64,
    pub token_nonce: u64,
}

impl FencingToken {
    pub fn new(generation: u64, token_nonce: u64) -> Self {
        Self {
            generation,
            token_nonce,
        }
    }

    /// Verifies that the provided token is strictly greater than or equal to expected generation.
    pub fn assert_valid(&self, current_generation: u64) -> Result<(), DomainError> {
        if self.generation < current_generation {
            Err(DomainError::Conflict(format!(
                "Stale fencing token generation {}: current generation is {}",
                self.generation, current_generation
            )))
        } else {
            Ok(())
        }
    }
}

/// Optimistic concurrency control version check for mutable entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptimisticVersionCheck {
    pub expected_version: u64,
}

impl OptimisticVersionCheck {
    pub fn new(expected_version: u64) -> Self {
        Self { expected_version }
    }

    pub fn verify(&self, actual_version: u64) -> Result<(), DomainError> {
        if self.expected_version != actual_version {
            Err(DomainError::Conflict(format!(
                "Optimistic lock conflict: expected version {}, found {}",
                self.expected_version, actual_version
            )))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fencing_token_validation() {
        let token = FencingToken::new(5, 42);
        assert!(token.assert_valid(5).is_ok());
        assert!(token.assert_valid(4).is_ok());
        assert!(token.assert_valid(6).is_err());
    }

    #[test]
    fn test_optimistic_version_check() {
        let check = OptimisticVersionCheck::new(3);
        assert!(check.verify(3).is_ok());
        assert!(check.verify(4).is_err());
    }
}
