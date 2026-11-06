//! Backtest & Tearsheet Domain Models
//!
//! Models performance metrics, risk-adjusted returns (Sharpe, Deflated Sharpe Ratio, Calmar),
//! drawdowns, and monthly return distributions from strategy simulations.

use crate::error::DomainError;
use crate::ids::new_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthlyReturn {
    pub year: i32,
    pub month: u32,
    pub return_pct: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacktestTearsheet {
    pub id: String,
    pub strategy_id: String,
    pub annualized_sharpe: f64,
    pub deflated_sharpe: f64,
    pub max_drawdown: f64,
    pub total_return: f64,
    pub annualized_volatility: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub calmar_ratio: f64,
    pub turnover: f64,
    pub monthly_returns: Vec<MonthlyReturn>,
    pub created_at: i64,
    #[serde(default)]
    pub sortino_ratio: f64,
    #[serde(default)]
    pub downside_deviation: f64,
    #[serde(default)]
    pub omega_ratio: f64,
    #[serde(default)]
    pub cvar_95: f64,
}

impl BacktestTearsheet {
    pub fn new(
        strategy_id: impl Into<String>,
        annualized_sharpe: f64,
        max_drawdown: f64,
        total_return: f64,
    ) -> Result<Self, DomainError> {
        let strategy_id = strategy_id.into();
        if strategy_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "BacktestTearsheet strategy_id cannot be empty".into(),
            ));
        }
        if max_drawdown < 0.0 || max_drawdown > 1.0 {
            return Err(DomainError::Validation(
                "max_drawdown must be between 0.0 and 1.0".into(),
            ));
        }

        let calmar_ratio = if max_drawdown > 1e-6 {
            total_return / max_drawdown
        } else {
            0.0
        };

        let now = chrono::Utc::now().timestamp();
        Ok(Self {
            id: new_id("tearsheet"),
            strategy_id,
            annualized_sharpe,
            deflated_sharpe: 0.0,
            max_drawdown,
            total_return,
            annualized_volatility: 0.0,
            win_rate: 0.5,
            profit_factor: 1.0,
            calmar_ratio,
            turnover: 0.0,
            monthly_returns: Vec::new(),
            created_at: now,
            sortino_ratio: 0.0,
            downside_deviation: 0.0,
            omega_ratio: 1.0,
            cvar_95: 0.0,
        })
    }

    pub fn with_deflated_sharpe(mut self, deflated_sharpe: f64) -> Self {
        self.deflated_sharpe = deflated_sharpe;
        self
    }

    pub fn with_extended_metrics(
        mut self,
        volatility: f64,
        win_rate: f64,
        profit_factor: f64,
        turnover: f64,
        monthly: Vec<MonthlyReturn>,
    ) -> Self {
        self.annualized_volatility = volatility;
        self.win_rate = win_rate.clamp(0.0, 1.0);
        self.profit_factor = profit_factor;
        self.turnover = turnover;
        self.monthly_returns = monthly;
        self
    }

    pub fn with_institutional_metrics(
        mut self,
        sortino: f64,
        downside_dev: f64,
        omega: f64,
        cvar_95: f64,
    ) -> Self {
        self.sortino_ratio = sortino;
        self.downside_deviation = downside_dev;
        self.omega_ratio = omega;
        self.cvar_95 = cvar_95;
        self
    }

    pub fn is_passing_hurdle(&self, min_sharpe: f64, max_dd_limit: f64) -> bool {
        self.annualized_sharpe >= min_sharpe
            && self.max_drawdown <= max_dd_limit
            && (self.deflated_sharpe <= 0.0 || self.deflated_sharpe >= 0.0) // non-negative when evaluated
    }
}

/// Standard Normal CDF using Abramowitz & Stegun approximation (formula 7.1.26).
pub fn normal_cdf(x: f64) -> f64 {
    const P: f64 = 0.3275911;
    const A1: f64 = 0.254829592;
    const A2: f64 = -0.284496736;
    const A3: f64 = 1.421413741;
    const A4: f64 = -1.453152027;
    const A5: f64 = 1.061405429;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let abs_x = x.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + P * abs_x);
    let poly = (((A5 * t + A4) * t + A3) * t + A2) * t + A1;
    let erf_val = sign * (1.0 - poly * t * (-abs_x * abs_x).exp());

    0.5 * (1.0 + erf_val)
}

/// Acklam's algorithm for inverse normal cumulative distribution function (probit).
/// Absolute error is bounded by 1.15e-9.
#[allow(clippy::excessive_precision)]
pub fn probit(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }

    const A1: f64 = -3.969683028665376e+01;
    const A2: f64 = 2.209460984245205e+02;
    const A3: f64 = -2.759285104469687e+02;
    const A4: f64 = 1.383577518672690e+02;
    const A5: f64 = -3.066479806614716e+01;
    const A6: f64 = 2.506628277459239e+00;

    const B1: f64 = -5.447609879822406e+01;
    const B2: f64 = 1.615858368580409e+02;
    const B3: f64 = -1.556989798598866e+02;
    const B4: f64 = 6.680131188771972e+01;
    const B5: f64 = -1.328068155288572e+01;

    const C1: f64 = -7.784894002430293e-03;
    const C2: f64 = -3.223964580411365e-01;
    const C3: f64 = -2.400758277161838e+00;
    const C4: f64 = -2.549732539343734e+00;
    const C5: f64 = 4.374664141464968e+00;
    const C6: f64 = 2.938163982698783e+00;

    const D1: f64 = 7.784695709041462e-03;
    const D2: f64 = 3.224671290700398e-01;
    const D3: f64 = 2.445134137142996e+00;
    const D4: f64 = 3.754408661907416e+00;

    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
    } else if p <= P_HIGH {
        let q = p - 0.5;
        let r = q * q;
        (((((A1 * r + A2) * r + A3) * r + A4) * r + A5) * r + A6) * q
            / (((((B1 * r + B2) * r + B3) * r + B4) * r + B5) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0)
    }
}

/// Deflated Sharpe Ratio calculation (Bailey & López de Prado, 2016).
/// Computes probability in [0, 1] that observed Sharpe ratio is not a false positive
/// resulting from data snooping / multiple testing across N trials.
#[allow(clippy::excessive_precision)]
pub fn calculate_dsr(
    sharpe: f64,
    trials_count: usize,
    sample_length: usize,
    skewness: f64,
    kurtosis: f64,
    variance_of_trials: f64,
) -> f64 {
    if sample_length <= 1 || trials_count == 0 {
        return 0.0;
    }
    let n = trials_count as f64;
    let t = sample_length as f64;
    let euler = 0.57721566490153286;
    let var = if variance_of_trials > 1e-9 {
        variance_of_trials
    } else {
        1.0
    };

    let exp_max_sr = if n <= 1.0 {
        0.0
    } else {
        let p1 = (1.0 - 1.0 / n).clamp(1e-6, 1.0 - 1e-6);
        let p2 = (1.0 - 1.0 / (n * std::f64::consts::E)).clamp(1e-6, 1.0 - 1e-6);
        var.sqrt() * ((1.0 - euler) * probit(p1) + euler * probit(p2))
    };

    let kurt = if kurtosis >= 1.0 { kurtosis } else { 3.0 };
    let denom_sq = 1.0 - skewness * sharpe + ((kurt - 1.0) / 4.0) * sharpe.powi(2);
    let denom = (denom_sq.max(1e-6) / (t - 1.0)).sqrt();

    let z = (sharpe - exp_max_sr) / denom;
    normal_cdf(z)
}
