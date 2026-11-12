//! Alpha Factor Domain Models
//!
//! Models quantitative alpha signals, statistical hypotheses, and predictive metrics
//! (Information Coefficient, Information Ratio, factor decay, and lookahead validation).

use custos_domain::DomainError;
use custos_domain::new_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlphaFactor {
    pub id: String,
    pub name: String,
    pub hypothesis: String,
    pub formula: String,
    pub universe: String,
    pub ic_mean: f64,
    pub ic_std: f64,
    pub ir: f64,
    pub decay_days: u32,
    pub lookahead_clean: bool,
    pub tags: Vec<String>,
    pub created_at: i64,
}

impl AlphaFactor {
    pub fn new(
        name: impl Into<String>,
        hypothesis: impl Into<String>,
        formula: impl Into<String>,
        universe: impl Into<String>,
        decay_days: u32,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let hypothesis = hypothesis.into();
        let formula = formula.into();
        let universe = universe.into();

        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "AlphaFactor name cannot be empty".into(),
            ));
        }
        if formula.trim().is_empty() {
            return Err(DomainError::Validation(
                "AlphaFactor formula cannot be empty".into(),
            ));
        }
        if decay_days == 0 {
            return Err(DomainError::Validation(
                "AlphaFactor decay_days must be positive".into(),
            ));
        }

        let now = chrono::Utc::now().timestamp();
        Ok(Self {
            id: new_id("factor"),
            name,
            hypothesis,
            formula,
            universe,
            ic_mean: 0.0,
            ic_std: 0.0,
            ir: 0.0,
            decay_days,
            lookahead_clean: false,
            tags: Vec::new(),
            created_at: now,
        })
    }

    pub fn with_metrics(mut self, ic_mean: f64, ic_std: f64, lookahead_clean: bool) -> Self {
        self.ic_mean = ic_mean;
        self.ic_std = ic_std;
        self.ir = if ic_std > 1e-9 { ic_mean / ic_std } else { 0.0 };
        self.lookahead_clean = lookahead_clean;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn mark_lookahead_checked(&mut self, is_clean: bool) {
        self.lookahead_clean = is_clean;
    }

    pub fn is_viable(&self) -> bool {
        self.ic_mean.abs() >= 0.02 && self.ir >= 0.5 && self.lookahead_clean
    }
}

/// Static analysis scanner for lookahead bias and future data leakage in factor formulas.
pub fn detect_lookahead_bias(formula: &str) -> Result<(), DomainError> {
    let lower = formula.to_ascii_lowercase();
    let forbidden_patterns = [
        ("shift(-", "negative lag / future shift"),
        ("shift (-", "negative lag / future shift"),
        ("lead(", "forward lead operator"),
        ("lead (", "forward lead operator"),
        ("future_", "future-dated column reference"),
        ("next_close", "unlagged future price reference"),
        ("next_day", "future day reference"),
        ("forward_return", "target leakage in feature formula"),
        ("target_return", "target leakage in feature formula"),
    ];

    for (pattern, reason) in forbidden_patterns {
        if lower.contains(pattern) {
            return Err(DomainError::Validation(format!(
                "Lookahead bias detected: pattern '{pattern}' indicates {reason}"
            )));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactorClassification {
    Alive,
    Candidate,
    Reversed,
    Decaying,
    Dead,
    Insufficient,
}

pub struct BenjaminiHochberg;

impl BenjaminiHochberg {
    /// Compute adjusted q-values from p-values using the Benjamini-Hochberg FDR procedure.
    pub fn compute_q_values(p_values: &[f64]) -> Vec<f64> {
        let m = p_values.len();
        if m == 0 {
            return Vec::new();
        }

        let mut indexed: Vec<(usize, f64)> = p_values.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut q_values = vec![1.0; m];
        let mut min_q: f64 = 1.0;

        for rank in (1..=m).rev() {
            let (orig_idx, p_val) = indexed[rank - 1];
            let q = (p_val * m as f64) / rank as f64;
            min_q = min_q.min(q).min(1.0);
            q_values[orig_idx] = min_q;
        }

        q_values
    }

    /// Classify factor predictive durability given in-sample and out-of-sample metrics.
    pub fn classify_factor(
        train_ic: f64,
        test_ic: f64,
        q_value: f64,
        fdr_threshold: f64,
        min_abs_oos_ic: f64,
        obs_count: usize,
    ) -> FactorClassification {
        if obs_count < 10 {
            return FactorClassification::Insufficient;
        }

        let same_sign = train_ic * test_ic > 0.0;
        let significant = q_value <= fdr_threshold;
        let usable = test_ic.abs() >= min_abs_oos_ic;

        if same_sign && significant && usable {
            FactorClassification::Alive
        } else if !same_sign && significant && usable {
            FactorClassification::Reversed
        } else if same_sign && usable {
            FactorClassification::Candidate
        } else if same_sign && test_ic.abs() < train_ic.abs() * 0.5 {
            FactorClassification::Decaying
        } else {
            FactorClassification::Dead
        }
    }
}

/// Compute Spearman Rank Correlation (Information Coefficient) between factor values and forward returns.
pub fn compute_rank_ic(factor_values: &[f64], forward_returns: &[f64]) -> f64 {
    let n = factor_values.len();
    if n < 3 || n != forward_returns.len() {
        return 0.0;
    }

    fn to_ranks(v: &[f64]) -> Vec<f64> {
        let mut indexed: Vec<(usize, f64)> = v.iter().copied().enumerate().collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut ranks = vec![0.0; v.len()];
        for (rank, &(idx, _)) in indexed.iter().enumerate() {
            ranks[idx] = (rank + 1) as f64;
        }
        ranks
    }

    let r_x = to_ranks(factor_values);
    let r_y = to_ranks(forward_returns);

    let mean_x = (n as f64 + 1.0) / 2.0;
    let mean_y = mean_x;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n {
        let dx = r_x[i] - mean_x;
        let dy = r_y[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom > 1e-12 {
        cov / denom
    } else {
        0.0
    }
}
