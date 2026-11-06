//! Strongly-typed Data Transfer Objects (DTOs) for Quantitative Trading RPC API.
//!
//! Enforces compile-time schema validation and eliminates manual untyped JSON navigation.

use custos_domain::quant::{MonthlyReturn, OhlcvBar, TradeFillRecord};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Backtesting & Simulation DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitBacktestRequest {
    pub strategy_id: String,
    pub task_id: String,
    pub formula: Option<String>,
    pub bars: Option<Vec<OhlcvBarDto>>,
    pub symbol: Option<String>,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default = "default_capital")]
    pub initial_capital: f64,
    #[serde(default = "default_commission")]
    pub commission: f64,
    #[serde(default = "default_slippage")]
    pub slippage_bps: f64,
    #[serde(default = "default_leverage")]
    pub leverage: f64,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub trailing_stop_pct: Option<f64>,
    pub trailing_activation_pct: Option<f64>,
    #[serde(default = "default_fast_period")]
    pub fast_period: usize,
    #[serde(default = "default_slow_period")]
    pub slow_period: usize,
    #[serde(default = "default_target_weight")]
    pub target_weight: f64,
    #[serde(default)]
    pub annualized_sharpe: f64,
    #[serde(default = "default_max_drawdown")]
    pub max_drawdown: f64,
    #[serde(default = "default_total_return")]
    pub total_return: f64,
    #[serde(default = "default_trials")]
    pub trials_count: usize,
    #[serde(default = "default_sample_length")]
    pub sample_length: usize,
    #[serde(default)]
    pub skewness: f64,
    #[serde(default = "default_kurtosis")]
    pub kurtosis: f64,
    #[serde(default = "default_variance")]
    pub variance_of_trials: f64,
    #[serde(default = "default_volatility")]
    pub volatility: f64,
    #[serde(default = "default_win_rate")]
    pub win_rate: f64,
    #[serde(default = "default_profit_factor")]
    pub profit_factor: f64,
    #[serde(default = "default_turnover")]
    pub turnover: f64,
    pub monthly_returns: Option<Vec<MonthlyReturnDto>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetTearsheetRequest {
    pub strategy_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GridEvaluateRequest {
    #[serde(default = "default_grid_symbol")]
    pub symbol: String,
    #[serde(default = "default_grid_lower")]
    pub lower_price: f64,
    #[serde(default = "default_grid_upper")]
    pub upper_price: f64,
    #[serde(default = "default_grid_count")]
    pub grid_count: usize,
    #[serde(default = "default_grid_mode")]
    pub mode: String,
    #[serde(default = "default_grid_capital")]
    pub total_capital: f64,
    #[serde(default = "default_grid_current")]
    pub current_price: f64,
    pub stop_loss_price: Option<f64>,
    pub take_profit_price: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComputeIndicatorsRequest {
    pub symbol: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default = "default_indicator_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComputePboRequest {
    pub score_matrix: Vec<Vec<f64>>,
    #[serde(default = "default_max_combinations")]
    pub max_combinations: usize,
}

// ---------------------------------------------------------------------------
// Orders & Execution DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct OrderSpecDto {
    pub symbol: String,
    #[serde(default = "default_order_direction")]
    pub direction: String,
    #[serde(default = "default_order_quantity")]
    pub quantity: f64,
    #[serde(default = "default_order_target_weight")]
    pub target_weight: f64,
    pub limit_price: Option<f64>,
    pub estimated_slippage_bps: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrdersRequest {
    pub strategy_id: String,
    pub task_id: String,
    pub orders: Vec<OrderSpecDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListOrdersRequest {
    pub strategy_id: Option<String>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApproveOrderRequest {
    pub order_id: String,
    pub permit_id: String,
    #[serde(default = "default_task_id")]
    pub task_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KillSwitchRequest {
    #[serde(default = "default_kill_switch_reason")]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetOrderbookRequest {
    #[serde(default = "default_orderbook_symbol")]
    pub symbol: String,
    #[serde(default = "default_orderbook_depth")]
    pub depth: usize,
}

// ---------------------------------------------------------------------------
// Market Data & Risk Gate DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverFactorsRequest {
    #[serde(default = "default_factor_symbol")]
    pub symbol: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IngestMarketBarsRequest {
    pub symbol: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    pub bars: Vec<OhlcvBarDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetMarketBarsRequest {
    pub symbol: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default = "default_bars_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvaluateGateRequest {
    pub symbol: String,
    #[serde(default = "default_order_direction")]
    pub direction: String,
    #[serde(default)]
    pub quantity: f64,
    #[serde(default)]
    pub estimated_price: f64,
    #[serde(default)]
    pub current_drawdown: f64,
    #[serde(default)]
    pub consecutive_losses: usize,
    #[serde(default)]
    pub daily_executed_notional: f64,
    #[serde(default)]
    pub is_risk_reducing: bool,
    #[serde(default = "default_max_order_notional")]
    pub max_order_notional: f64,
    #[serde(default = "default_max_daily_notional")]
    pub max_daily_notional: f64,
    #[serde(default = "default_max_drawdown_limit")]
    pub max_drawdown_limit: f64,
    #[serde(default = "default_max_consecutive_losses")]
    pub max_consecutive_losses: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ComputeReviewRequest {
    #[serde(default = "default_strategy_id")]
    pub strategy_id: String,
    #[serde(default = "default_review_symbol")]
    pub symbol: String,
    pub fills: Vec<TradeFillRecord>,
}

// ---------------------------------------------------------------------------
// Shared Primitives & Conversions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvBarDto {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl From<OhlcvBarDto> for OhlcvBar {
    fn from(b: OhlcvBarDto) -> Self {
        OhlcvBar::new(b.timestamp, b.open, b.high, b.low, b.close, b.volume)
    }
}

impl From<&OhlcvBarDto> for OhlcvBar {
    fn from(b: &OhlcvBarDto) -> Self {
        OhlcvBar::new(b.timestamp, b.open, b.high, b.low, b.close, b.volume)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyReturnDto {
    pub year: i32,
    pub month: u32,
    pub return_pct: f64,
}

impl From<MonthlyReturnDto> for MonthlyReturn {
    fn from(m: MonthlyReturnDto) -> Self {
        MonthlyReturn {
            year: m.year,
            month: m.month,
            return_pct: m.return_pct,
        }
    }
}

// Default helper functions
fn default_timeframe() -> String {
    "1d".into()
}
fn default_capital() -> f64 {
    100_000.0
}
fn default_commission() -> f64 {
    0.0005
}
fn default_slippage() -> f64 {
    2.0
}
fn default_leverage() -> f64 {
    1.0
}
fn default_fast_period() -> usize {
    5
}
fn default_slow_period() -> usize {
    20
}
fn default_target_weight() -> f64 {
    1.0
}
fn default_max_drawdown() -> f64 {
    0.15
}
fn default_total_return() -> f64 {
    0.25
}
fn default_trials() -> usize {
    1
}
fn default_sample_length() -> usize {
    252
}
fn default_kurtosis() -> f64 {
    3.0
}
fn default_variance() -> f64 {
    1.0
}
fn default_volatility() -> f64 {
    0.15
}
fn default_win_rate() -> f64 {
    0.55
}
fn default_profit_factor() -> f64 {
    1.5
}
fn default_turnover() -> f64 {
    0.2
}
fn default_grid_symbol() -> String {
    "BTCUSDT".into()
}
fn default_grid_lower() -> f64 {
    50_000.0
}
fn default_grid_upper() -> f64 {
    80_000.0
}
fn default_grid_count() -> usize {
    10
}
fn default_grid_mode() -> String {
    "arithmetic".into()
}
fn default_grid_capital() -> f64 {
    10_000.0
}
fn default_grid_current() -> f64 {
    65_000.0
}
fn default_indicator_limit() -> usize {
    200
}
fn default_max_combinations() -> usize {
    2000
}
fn default_order_direction() -> String {
    "buy".into()
}
fn default_order_quantity() -> f64 {
    100.0
}
fn default_order_target_weight() -> f64 {
    0.05
}
fn default_task_id() -> String {
    "quant_task".into()
}
fn default_kill_switch_reason() -> String {
    "Manual Emergency Kill-Switch Activated".into()
}
fn default_orderbook_symbol() -> String {
    "BTC/USDT".into()
}
fn default_orderbook_depth() -> usize {
    5
}
fn default_factor_symbol() -> String {
    "SPY".into()
}
fn default_bars_limit() -> usize {
    500
}
fn default_max_order_notional() -> f64 {
    50_000.0
}
fn default_max_daily_notional() -> f64 {
    250_000.0
}
fn default_max_drawdown_limit() -> f64 {
    0.20
}
fn default_max_consecutive_losses() -> usize {
    5
}
fn default_strategy_id() -> String {
    "strategy_default".into()
}
fn default_review_symbol() -> String {
    "ALL".into()
}
