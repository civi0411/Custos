//! Execution Budget Tracker
//!
//! Prevents runaway model loops and controls computational/financial costs.

use serde::{Deserialize, Serialize};

use crate::types::DomainError;

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
        if self.settled_spans + self.reserved_spans + spans > self.max_spans {
            return Err(DomainError::BudgetExceeded("Span quota exceeded".into()));
        }
        if self.settled_tokens + self.reserved_tokens + tokens > self.max_tokens {
            return Err(DomainError::BudgetExceeded("Token quota exceeded".into()));
        }
        self.reserved_spans += spans;
        self.reserved_tokens += tokens;
        Ok(())
    }

    pub fn settle(&mut self, actual_spans: u32, actual_tokens: u64) -> Result<(), DomainError> {
        self.reserved_spans = self.reserved_spans.saturating_sub(actual_spans);
        self.reserved_tokens = self.reserved_tokens.saturating_sub(actual_tokens);
        self.settled_spans += actual_spans;
        self.settled_tokens += actual_tokens;
        Ok(())
    }
}
