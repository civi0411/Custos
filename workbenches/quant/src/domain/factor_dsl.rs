//! Safe Zero-I/O Factor Expression Evaluator & Strategy Rule Bridge
//!
//! Compiles validated lookahead-free factor formulas into executable `StrategyRule`
//! instances and cross-sectional factor series without arbitrary code execution.

use super::engine::{DualMovingAverageRule, OhlcvBar, SignalAction, StrategyRule};
use super::factor::detect_lookahead_bias;
use custos_contracts::DomainError;

#[derive(Debug, Clone, PartialEq)]
pub enum FactorExpr {
    /// Rate of change: close[t] / close[t - n] - 1
    Momentum { period: usize },
    /// Rolling close volatility normalized by mean: std(close, n) / mean(close, n)
    Volatility { period: usize },
    /// Mean-reversion z-score: -(close[t] - sma(n)) / std(n)
    MeanReversionZScore { period: usize },
    /// Wilder RSI mean-reversion signal: (50 - rsi(n)) / 50
    RsiMeanReversion { period: usize },
    /// Dual moving average crossover: sma(fast) > sma(slow)
    DualMaCrossover { fast: usize, slow: usize },
    /// Volume-confirmed momentum: roc(period) * (volume / sma_vol(period))
    VolumeMomentum { period: usize },
}

fn extract_numbers(formula: &str) -> Vec<usize> {
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in formula.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            if let Ok(v) = cur.parse::<usize>() {
                if v > 0 {
                    nums.push(v);
                }
            }
            cur.clear();
        }
    }
    if !cur.is_empty() {
        if let Ok(v) = cur.parse::<usize>() {
            if v > 0 {
                nums.push(v);
            }
        }
    }
    nums
}

impl FactorExpr {
    /// Parse a validated formula string into a deterministic zero-I/O `FactorExpr`.
    pub fn parse(formula: &str) -> Result<Self, DomainError> {
        detect_lookahead_bias(formula)?;
        let lower = formula.trim().to_ascii_lowercase();
        let nums = extract_numbers(&lower);

        if lower.contains("rsi") {
            let period = nums.first().copied().unwrap_or(14).clamp(2, 100);
            Ok(Self::RsiMeanReversion { period })
        } else if lower.contains("zscore")
            || lower.contains("mean_rev")
            || lower.contains("reversion")
        {
            let period = nums.first().copied().unwrap_or(20).clamp(3, 200);
            Ok(Self::MeanReversionZScore { period })
        } else if lower.contains("vol") && (lower.contains("mom") || lower.contains("volume")) {
            let period = nums.first().copied().unwrap_or(10).clamp(2, 200);
            Ok(Self::VolumeMomentum { period })
        } else if lower.contains("std") || lower.contains("volatility") || lower.contains("atr") {
            let period = nums.first().copied().unwrap_or(10).clamp(3, 200);
            Ok(Self::Volatility { period })
        } else if lower.contains("sma")
            || lower.contains("ema")
            || lower.contains("dual_ma")
            || lower.contains("cross")
        {
            let fast = nums.first().copied().unwrap_or(5).clamp(2, 100);
            let slow = nums
                .get(1)
                .copied()
                .unwrap_or((fast * 3).max(fast + 2))
                .clamp(fast + 1, 250);
            Ok(Self::DualMaCrossover { fast, slow })
        } else {
            let period = nums.first().copied().unwrap_or(5).clamp(1, 200);
            Ok(Self::Momentum { period })
        }
    }

    /// Evaluate the factor value at index `idx` using strictly `history[..=idx]`.
    pub fn eval_at(&self, idx: usize, history: &[OhlcvBar]) -> Option<f64> {
        if idx >= history.len() {
            return None;
        }
        match *self {
            Self::Momentum { period } => {
                if idx < period {
                    return None;
                }
                let prev = history[idx - period].close;
                if prev.abs() < 1e-12 {
                    return None;
                }
                Some(history[idx].close / prev - 1.0)
            }
            Self::Volatility { period } => {
                if idx + 1 < period {
                    return None;
                }
                let slice = &history[idx + 1 - period..=idx];
                let mean = slice.iter().map(|b| b.close).sum::<f64>() / period as f64;
                if mean.abs() < 1e-12 {
                    return None;
                }
                let var =
                    slice.iter().map(|b| (b.close - mean).powi(2)).sum::<f64>() / period as f64;
                Some(var.sqrt() / mean)
            }
            Self::MeanReversionZScore { period } => {
                if idx + 1 < period {
                    return None;
                }
                let slice = &history[idx + 1 - period..=idx];
                let mean = slice.iter().map(|b| b.close).sum::<f64>() / period as f64;
                let var =
                    slice.iter().map(|b| (b.close - mean).powi(2)).sum::<f64>() / period as f64;
                let std = var.sqrt();
                if std < 1e-12 {
                    return Some(0.0);
                }
                Some(-(history[idx].close - mean) / std)
            }
            Self::RsiMeanReversion { period } => {
                if idx < period {
                    return None;
                }
                let rsi = super::portal::compute_wilder_rsi_slice(&history[..=idx], period)?;
                Some((50.0 - rsi) / 50.0)
            }
            Self::DualMaCrossover { fast, slow } => {
                if idx + 1 < slow {
                    return None;
                }
                let fast_sma = history[idx + 1 - fast..=idx]
                    .iter()
                    .map(|b| b.close)
                    .sum::<f64>()
                    / fast as f64;
                let slow_sma = history[idx + 1 - slow..=idx]
                    .iter()
                    .map(|b| b.close)
                    .sum::<f64>()
                    / slow as f64;
                if slow_sma.abs() < 1e-12 {
                    return None;
                }
                Some((fast_sma - slow_sma) / slow_sma)
            }
            Self::VolumeMomentum { period } => {
                if idx < period {
                    return None;
                }
                let prev = history[idx - period].close;
                if prev.abs() < 1e-12 {
                    return None;
                }
                let roc = history[idx].close / prev - 1.0;
                let avg_vol = history[idx + 1 - period..=idx]
                    .iter()
                    .map(|b| b.volume)
                    .sum::<f64>()
                    / period as f64;
                let rel_vol = if avg_vol > 1e-12 {
                    (history[idx].volume / avg_vol).clamp(0.2, 5.0)
                } else {
                    1.0
                };
                Some(roc * rel_vol)
            }
        }
    }

    /// Evaluate factor values across all bars, returning aligned `(bar_idx, value)` pairs.
    pub fn eval_series(&self, bars: &[OhlcvBar]) -> Vec<(usize, f64)> {
        (0..bars.len())
            .filter_map(|i| self.eval_at(i, bars).map(|v| (i, v)))
            .collect()
    }
}

/// Converts any parsed `FactorExpr` into a deterministic zero-lookahead `StrategyRule`.
pub struct FormulaStrategyRule {
    expr: FactorExpr,
    target_pct: f64,
    allow_short: bool,
}

impl FormulaStrategyRule {
    pub fn from_formula(
        formula: &str,
        target_pct: f64,
        allow_short: bool,
    ) -> Result<Box<dyn StrategyRule>, DomainError> {
        let expr = FactorExpr::parse(formula)?;
        if let FactorExpr::DualMaCrossover { fast, slow } = expr {
            if !allow_short {
                return Ok(Box::new(DualMovingAverageRule::new(fast, slow, target_pct)));
            }
        }
        Ok(Box::new(Self {
            expr,
            target_pct: target_pct.clamp(0.05, 1.0),
            allow_short,
        }))
    }
}

impl StrategyRule for FormulaStrategyRule {
    fn on_bar(&mut self, bar_idx: usize, history: &[OhlcvBar]) -> SignalAction {
        let Some(score) = self.expr.eval_at(bar_idx, history) else {
            return SignalAction::Hold;
        };
        if score > 0.002 {
            SignalAction::TargetPercent(self.target_pct)
        } else if score < -0.002 {
            if self.allow_short {
                SignalAction::TargetPercent(-self.target_pct)
            } else {
                SignalAction::TargetPercent(0.0)
            }
        } else {
            SignalAction::Hold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factor_expr_rejects_lookahead() {
        assert!(FactorExpr::parse("close.shift(-1) / close").is_err());
        assert!(FactorExpr::parse("lead(close, 2)").is_err());
    }

    #[test]
    fn test_factor_expr_parses_and_evaluates_variants() {
        let bars: Vec<OhlcvBar> = (0..30)
            .map(|i| {
                OhlcvBar::new(
                    i * 86400,
                    100.0 + i as f64,
                    101.0 + i as f64,
                    99.0 + i as f64,
                    100.5 + i as f64,
                    1000.0,
                )
            })
            .collect();
        for formula in [
            "close / delay(close, 5) - 1",
            "std(close, 10) / sma(close, 10)",
            "zscore(close, 15)",
            "rsi(14)",
            "sma(5) > sma(15)",
            "volume_momentum(8)",
        ] {
            let expr = FactorExpr::parse(formula).expect("valid formula");
            assert!(
                !expr.eval_series(&bars).is_empty(),
                "empty series for {formula}"
            );
        }
    }
}
