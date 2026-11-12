//! Execution Budget Tracker
//!
//! Controls token and span quotas and enforces Anti-Gaming Invariants (Custos.md §16.3).

use crate::error::DomainError;
use serde::{Deserialize, Serialize};

/// Abstract headroom signals provided to models to prevent gaming behavior (Custos.md §16.3 Anti-Gaming Invariant).
/// The model is NEVER told the exact token numbers remaining.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Headroom {
    /// Plentiful budget (> 50% remaining)
    Abundant,
    /// Moderate budget (15% - 50% remaining)
    Constrained,
    /// Low budget (< 15% remaining), wrap up immediately
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationToken {
    pub token_id: String,
    pub task_id: String,
    pub reserved_spans: u32,
    pub reserved_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_spans: u32,
    pub max_tokens: u64,
    pub max_cost_cents: u32,
    pub reserved_spans: u32,
    pub reserved_tokens: u64,
    pub settled_spans: u32,
    pub settled_tokens: u64,
}

impl Budget {
    pub fn new(max_spans: u32, max_tokens: u64, max_cost_cents: u32) -> Self {
        Self {
            max_spans,
            max_tokens,
            max_cost_cents,
            reserved_spans: 0,
            reserved_tokens: 0,
            settled_spans: 0,
            settled_tokens: 0,
        }
    }

    pub fn reserve(&mut self, spans: u32, tokens: u64) -> Result<(), DomainError> {
        let committed_spans = self
            .settled_spans
            .checked_add(self.reserved_spans)
            .and_then(|value| value.checked_add(spans))
            .ok_or_else(|| DomainError::BudgetExceeded("Span quota overflow".into()))?;
        if committed_spans > self.max_spans {
            return Err(DomainError::BudgetExceeded("Span quota exceeded".into()));
        }
        let committed_tokens = self
            .settled_tokens
            .checked_add(self.reserved_tokens)
            .and_then(|value| value.checked_add(tokens))
            .ok_or_else(|| DomainError::BudgetExceeded("Token quota overflow".into()))?;
        if committed_tokens > self.max_tokens {
            return Err(DomainError::BudgetExceeded("Token quota exceeded".into()));
        }
        self.reserved_spans = self
            .reserved_spans
            .checked_add(spans)
            .ok_or_else(|| DomainError::BudgetExceeded("Reserved span counter overflow".into()))?;
        self.reserved_tokens = self
            .reserved_tokens
            .checked_add(tokens)
            .ok_or_else(|| DomainError::BudgetExceeded("Reserved token counter overflow".into()))?;
        Ok(())
    }

    pub fn settle(&mut self, actual_spans: u32, actual_tokens: u64) -> Result<(), DomainError> {
        if actual_spans > self.reserved_spans || actual_tokens > self.reserved_tokens {
            return Err(DomainError::InvariantViolation(format!(
                "Cannot settle {actual_spans} spans and {actual_tokens} tokens from reservation of {} spans and {} tokens",
                self.reserved_spans, self.reserved_tokens
            )));
        }

        let settled_spans = self
            .settled_spans
            .checked_add(actual_spans)
            .ok_or_else(|| DomainError::BudgetExceeded("Settled span counter overflow".into()))?;
        let settled_tokens = self
            .settled_tokens
            .checked_add(actual_tokens)
            .ok_or_else(|| DomainError::BudgetExceeded("Settled token counter overflow".into()))?;
        if settled_spans > self.max_spans || settled_tokens > self.max_tokens {
            return Err(DomainError::BudgetExceeded(
                "Settlement would exceed the configured budget".into(),
            ));
        }

        self.reserved_spans -= actual_spans;
        self.reserved_tokens -= actual_tokens;
        self.settled_spans = settled_spans;
        self.settled_tokens = settled_tokens;
        Ok(())
    }

    pub fn refund(&mut self, spans: u32, tokens: u64) -> Result<(), DomainError> {
        if spans > self.reserved_spans || tokens > self.reserved_tokens {
            return Err(DomainError::InvariantViolation(format!(
                "Cannot refund {spans} spans and {tokens} tokens from reservation of {} spans and {} tokens",
                self.reserved_spans, self.reserved_tokens
            )));
        }
        self.reserved_spans -= spans;
        self.reserved_tokens -= tokens;
        Ok(())
    }

    /// Computes the abstract headroom tier without exposing raw numbers to models (Anti-Gaming Invariant).
    pub fn headroom(&self) -> Headroom {
        let total = self.max_tokens;
        if total == 0 {
            return Headroom::Critical;
        }
        let used = self.settled_tokens.saturating_add(self.reserved_tokens);
        if used >= total {
            return Headroom::Critical;
        }
        let remaining = total - used;
        let pct = (remaining as f64) / (total as f64);
        if pct > 0.50 {
            Headroom::Abundant
        } else if pct >= 0.15 {
            Headroom::Constrained
        } else {
            Headroom::Critical
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_headroom_tiers_anti_gaming() {
        let mut budget = Budget::new(10, 1000, 100);
        assert_eq!(budget.headroom(), Headroom::Abundant);

        // Reserve 600 tokens (400 remaining = 40%) -> Constrained
        assert!(budget.reserve(1, 600).is_ok());
        assert_eq!(budget.headroom(), Headroom::Constrained);

        // Settle 600 tokens and reserve 300 more (100 remaining = 10%) -> Critical
        assert!(budget.settle(1, 600).is_ok());
        assert!(budget.reserve(1, 300).is_ok());
        assert_eq!(budget.headroom(), Headroom::Critical);

        // Refund 300 -> back to Constrained
        assert!(budget.refund(1, 300).is_ok());
        assert_eq!(budget.headroom(), Headroom::Constrained);
    }

    #[test]
    fn settlement_and_refund_reject_amounts_above_reservation_atomically() {
        let mut budget = Budget::new(10, 1_000, 100);
        assert!(budget.reserve(2, 400).is_ok());
        let before = budget.clone();

        assert!(matches!(
            budget.settle(3, 400),
            Err(DomainError::InvariantViolation(_))
        ));
        assert_eq!(budget.reserved_spans, before.reserved_spans);
        assert_eq!(budget.reserved_tokens, before.reserved_tokens);
        assert_eq!(budget.settled_spans, before.settled_spans);
        assert_eq!(budget.settled_tokens, before.settled_tokens);

        assert!(matches!(
            budget.refund(2, 401),
            Err(DomainError::InvariantViolation(_))
        ));
        assert_eq!(budget.reserved_spans, before.reserved_spans);
        assert_eq!(budget.reserved_tokens, before.reserved_tokens);
    }

    #[test]
    fn reserve_rejects_counter_overflow_without_mutation() {
        let mut budget = Budget::new(u32::MAX, u64::MAX, 0);
        budget.settled_spans = u32::MAX;
        budget.settled_tokens = u64::MAX;

        assert!(matches!(
            budget.reserve(1, 1),
            Err(DomainError::BudgetExceeded(_))
        ));
        assert_eq!(budget.reserved_spans, 0);
        assert_eq!(budget.reserved_tokens, 0);
    }
}
