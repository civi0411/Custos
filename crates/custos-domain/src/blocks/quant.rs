//! Quant Block (Domain Layer)
//!
//! Provides the core domain aggregates and value objects for the Quant Workbench:
//! alpha factors, backtesting tearsheets, risk breach gates, order tickets, and market data bars.

pub use crate::quant::backtest::{calculate_dsr, BacktestTearsheet, MonthlyReturn};
pub use crate::quant::contracts::{
    ExecutionStatus, FillSnapshot, OrderExecutionStrategy, OrderIntent, OrderSide, PositionSnapshot,
};
pub use crate::quant::engine::{BacktestConfig, NativeBacktestEngine, OhlcvBar};
pub use crate::quant::factor::{
    compute_rank_ic, detect_lookahead_bias, AlphaFactor, FactorClassification,
};
pub use crate::quant::gate::{DecisionContext, GateOutcome, PreTradeDecisionGate};
pub use crate::quant::grid::{GridConfig, GridMode, GridOrderAction, NativeGridEngine};
pub use crate::quant::lease::{LeaseGuard, LeaseRecord, LeaseTier};
pub use crate::quant::metrics::{compute_institutional_suite, InstitutionalMetrics};
pub use crate::quant::order::{OrderDirection, OrderTicket, OrderTicketStatus, OrderType};
pub use crate::quant::orderbook::{OrderBookLevel, OrderBookSnapshot};
pub use crate::quant::pbo::{calculate_pbo_cscv, PboResult};
pub use crate::quant::portal::{BarTimeframe, NativeDataPortal};
pub use crate::quant::review::{PostTradeAnalyzer, PostTradeReviewReport};
pub use crate::quant::risk::{ProtectionSpec, ProtectionState, RiskBreach, RiskPermitConfig};
