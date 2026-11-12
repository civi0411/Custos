use custos_domain::pipeline::DataInvariant;
use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

/// Quantitative Trading Invariant: Guarantees zero lookahead bias by enforcing closed bar boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosedBarInvariant {
    pub bar_open_ts: i64,
    pub duration_seconds: i64,
    pub reference_ts: i64,
}

impl ClosedBarInvariant {
    pub fn new(bar_open_ts: i64, duration_seconds: i64, reference_ts: i64) -> Self {
        Self {
            bar_open_ts,
            duration_seconds,
            reference_ts,
        }
    }
}

impl DataInvariant for ClosedBarInvariant {
    fn verify(&self) -> Result<(), DomainError> {
        if self.duration_seconds <= 0 {
            return Err(DomainError::Validation(
                "Bar duration must be positive".to_string(),
            ));
        }
        let bar_close_ts = self.bar_open_ts.saturating_add(self.duration_seconds);
        if bar_close_ts > self.reference_ts {
            return Err(DomainError::Validation(format!(
                "Lookahead violation: bar closes at {} which is after reference time {}",
                bar_close_ts, self.reference_ts
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closed_bar_invariant_lookahead_rejection() {
        let valid_inv = ClosedBarInvariant::new(1000, 300, 1300);
        assert!(valid_inv.verify().is_ok());

        let lookahead_inv = ClosedBarInvariant::new(1000, 300, 1299);
        assert!(lookahead_inv.verify().is_err());
    }
}

