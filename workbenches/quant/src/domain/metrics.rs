//! Institutional Performance & Risk Metrics
//!
//! Provides advanced statistical metrics for quantitative strategy evaluation:
//! Sortino ratio, Downside Deviation, Dual Beta (Bull/Bear), Up/Down Capture,
//! Omega ratio, and Tail Risk (VaR & Conditional VaR / Expected Shortfall).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstitutionalMetrics {
    pub sortino_ratio: f64,
    pub downside_deviation: f64,
    pub omega_ratio: f64,
    pub var_95: f64,
    pub cvar_95: f64,
    pub bull_beta: f64,
    pub bear_beta: f64,
    pub up_capture: f64,
    pub down_capture: f64,
}

/// Calculate downside deviation given periodic returns and minimum acceptable return (MAR)
pub fn calculate_downside_deviation(returns: &[f64], mar: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let sum_sq_diff: f64 = returns
        .iter()
        .map(|&r| if r < mar { (r - mar).powi(2) } else { 0.0 })
        .sum();
    (sum_sq_diff / returns.len() as f64).sqrt()
}

/// Calculate Sortino ratio annualized
pub fn calculate_sortino_ratio(mean_return: f64, downside_dev: f64, periods_per_year: f64) -> f64 {
    if downside_dev < 1e-8 {
        return 0.0;
    }
    (mean_return / downside_dev) * periods_per_year.sqrt()
}

/// Calculate Value at Risk (VaR) and Conditional VaR (CVaR / Expected Shortfall)
pub fn calculate_cvar(returns: &[f64], confidence: f64) -> (f64, f64) {
    if returns.len() < 5 {
        return (0.0, 0.0);
    }
    let mut sorted = returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let alpha = (1.0 - confidence).clamp(0.01, 0.50);
    let cutoff_idx = ((sorted.len() as f64 * alpha).floor() as usize).max(1);

    let var = -sorted[cutoff_idx - 1]; // loss is expressed as positive number
    let tail_losses = &sorted[..cutoff_idx];
    let cvar = -tail_losses.iter().sum::<f64>() / tail_losses.len() as f64;

    (var, cvar)
}

/// Calculate Omega Ratio: ratio of probability-weighted gains vs probability-weighted losses
pub fn calculate_omega_ratio(returns: &[f64], threshold: f64) -> f64 {
    if returns.is_empty() {
        return 1.0;
    }
    let mut total_gains = 0.0;
    let mut total_losses = 0.0;

    for &r in returns {
        if r > threshold {
            total_gains += r - threshold;
        } else {
            total_losses += threshold - r;
        }
    }

    if total_losses > 1e-8 {
        total_gains / total_losses
    } else if total_gains > 0.0 {
        10.0
    } else {
        1.0
    }
}

/// Calculate Dual Beta: Bull Beta (when benchmark > 0) and Bear Beta (when benchmark < 0)
pub fn calculate_dual_beta(portfolio_returns: &[f64], benchmark_returns: &[f64]) -> (f64, f64) {
    let n = portfolio_returns.len().min(benchmark_returns.len());
    if n < 5 {
        return (1.0, 1.0);
    }

    let mut bull_p = Vec::new();
    let mut bull_b = Vec::new();
    let mut bear_p = Vec::new();
    let mut bear_b = Vec::new();

    for i in 0..n {
        let p = portfolio_returns[i];
        let b = benchmark_returns[i];
        if b >= 0.0 {
            bull_p.push(p);
            bull_b.push(b);
        } else {
            bear_p.push(p);
            bear_b.push(b);
        }
    }

    let calc_beta = |p: &[f64], b: &[f64]| -> f64 {
        if p.len() < 3 {
            return 1.0;
        }
        let mean_p: f64 = p.iter().sum::<f64>() / p.len() as f64;
        let mean_b: f64 = b.iter().sum::<f64>() / b.len() as f64;
        let mut cov = 0.0;
        let mut var_b = 0.0;
        for i in 0..p.len() {
            cov += (p[i] - mean_p) * (b[i] - mean_b);
            var_b += (b[i] - mean_b).powi(2);
        }
        if var_b > 1e-8 {
            cov / var_b
        } else {
            1.0
        }
    };

    (calc_beta(&bull_p, &bull_b), calc_beta(&bear_p, &bear_b))
}

/// Calculate Up/Down Capture Ratios relative to benchmark returns
pub fn calculate_capture_ratios(
    portfolio_returns: &[f64],
    benchmark_returns: &[f64],
) -> (f64, f64) {
    let n = portfolio_returns.len().min(benchmark_returns.len());
    if n < 5 {
        return (1.0, 1.0);
    }

    let mut up_p_sum = 0.0;
    let mut up_b_sum = 0.0;
    let mut down_p_sum = 0.0;
    let mut down_b_sum = 0.0;

    for i in 0..n {
        let p = portfolio_returns[i];
        let b = benchmark_returns[i];
        if b > 0.0 {
            up_p_sum += p;
            up_b_sum += b;
        } else if b < 0.0 {
            down_p_sum += p;
            down_b_sum += b;
        }
    }

    let up_capture = if up_b_sum.abs() > 1e-8 {
        up_p_sum / up_b_sum
    } else {
        1.0
    };
    let down_capture = if down_b_sum.abs() > 1e-8 {
        down_p_sum / down_b_sum
    } else {
        1.0
    };

    (up_capture, down_capture)
}

/// Compute full institutional metrics suite from return series
pub fn compute_institutional_suite(
    returns: &[f64],
    benchmark: Option<&[f64]>,
    annualization: f64,
) -> InstitutionalMetrics {
    let mean_ret = if !returns.is_empty() {
        returns.iter().sum::<f64>() / returns.len() as f64
    } else {
        0.0
    };
    let downside_dev = calculate_downside_deviation(returns, 0.0);
    let sortino = calculate_sortino_ratio(mean_ret, downside_dev, annualization);
    let omega = calculate_omega_ratio(returns, 0.0);
    let (var_95, cvar_95) = calculate_cvar(returns, 0.95);

    let (bull_beta, bear_beta, up_capture, down_capture) = if let Some(bench) = benchmark {
        let (bull_b, bear_b) = calculate_dual_beta(returns, bench);
        let (up_c, down_c) = calculate_capture_ratios(returns, bench);
        (bull_b, bear_b, up_c, down_c)
    } else {
        (1.0, 1.0, 1.0, 1.0)
    };

    InstitutionalMetrics {
        sortino_ratio: sortino,
        downside_deviation: downside_dev * annualization.sqrt(),
        omega_ratio: omega,
        var_95,
        cvar_95,
        bull_beta,
        bear_beta,
        up_capture,
        down_capture,
    }
}
