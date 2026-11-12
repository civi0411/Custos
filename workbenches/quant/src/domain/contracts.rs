//! Quant Exchange Contracts & Execution Strategy Patterns
//!
//! Provides the normalized Domain Value Objects, Polymorphic Adapter Traits,
//! and Execution Strategies (Strategy Pattern) for order execution.

use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketType {
    Spot,
    Swap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Submitted,
    Partial,
    Filled,
    Cancelled,
    Rejected,
    Unknown,
}

/// Normalized Value Object for an Order Intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderIntent {
    pub symbol: String,
    pub side: OrderSide,
    pub quantity: f64,
    pub price: Option<f64>,
    pub market_type: MarketType,
    pub reduce_only: bool,
    pub client_order_id: String,
    pub fallback_client_order_id: Option<String>,
    pub leverage: f64,
}

impl OrderIntent {
    pub fn new(
        symbol: impl Into<String>,
        side: OrderSide,
        quantity: f64,
        client_order_id: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let symbol = symbol.into();
        let client_order_id = client_order_id.into();

        if symbol.trim().is_empty() {
            return Err(DomainError::Validation("Symbol cannot be empty".into()));
        }
        if quantity <= 0.0 {
            return Err(DomainError::Validation(
                "Order quantity must be positive".into(),
            ));
        }
        if client_order_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Client order ID cannot be empty".into(),
            ));
        }

        Ok(Self {
            symbol,
            side,
            quantity,
            price: None,
            market_type: MarketType::Swap,
            reduce_only: false,
            client_order_id,
            fallback_client_order_id: None,
            leverage: 1.0,
        })
    }

    pub fn with_limit_price(mut self, price: f64) -> Result<Self, DomainError> {
        if price <= 0.0 {
            return Err(DomainError::Validation(
                "Limit price must be positive".into(),
            ));
        }
        self.price = Some(price);
        Ok(self)
    }

    pub fn with_fallback_id(mut self, fallback_id: impl Into<String>) -> Self {
        self.fallback_client_order_id = Some(fallback_id.into());
        self
    }

    pub fn with_reduce_only(mut self, reduce_only: bool) -> Self {
        self.reduce_only = reduce_only;
        self
    }
}

/// Normalized Value Object for an execution snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillSnapshot {
    pub order_id: String,
    pub filled_qty: f64,
    pub avg_price: f64,
    pub status: ExecutionStatus,
    pub fee_quote: f64,
}

impl FillSnapshot {
    pub fn is_complete(&self, requested_qty: f64) -> bool {
        self.status == ExecutionStatus::Filled
            || (self.filled_qty >= requested_qty * 0.999999 && requested_qty > 0.0)
    }
}

/// Normalized Value Object for a position snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub symbol: String,
    pub side: OrderSide,
    pub base_qty: f64,
    pub avg_price: f64,
}

/// Stable polymorphic interface (Protocol/Trait) for venue adapters.
pub trait ExchangeOrderAdapter: Send + Sync {
    fn venue_id(&self) -> &'static str;
    fn place_market_order(&self, intent: &OrderIntent) -> Result<FillSnapshot, DomainError>;
    fn place_limit_order(&self, intent: &OrderIntent) -> Result<FillSnapshot, DomainError>;
    fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), DomainError>;
    fn wait_for_fill(
        &self,
        intent: &OrderIntent,
        order_id: &str,
        timeout_ms: u64,
    ) -> Result<FillSnapshot, DomainError>;
    fn query_position(&self, symbol: &str) -> Result<Option<PositionSnapshot>, DomainError>;
}

/// Strategy Pattern: Order Execution Strategy.
pub trait OrderExecutionStrategy {
    fn execute<A: ExchangeOrderAdapter>(
        &self,
        adapter: &A,
        intent: &OrderIntent,
    ) -> Result<FillSnapshot, DomainError>;
}

/// Strategy 1: Immediate Market Order Execution.
pub struct MarketOrderExecutor;

impl OrderExecutionStrategy for MarketOrderExecutor {
    fn execute<A: ExchangeOrderAdapter>(
        &self,
        adapter: &A,
        intent: &OrderIntent,
    ) -> Result<FillSnapshot, DomainError> {
        adapter.place_market_order(intent)
    }
}

/// Strategy 2: Resting Limit Order Execution.
pub struct RestingLimitExecutor;

impl OrderExecutionStrategy for RestingLimitExecutor {
    fn execute<A: ExchangeOrderAdapter>(
        &self,
        adapter: &A,
        intent: &OrderIntent,
    ) -> Result<FillSnapshot, DomainError> {
        if intent.price.is_none() || intent.price.unwrap_or(0.0) <= 0.0 {
            return Err(DomainError::Validation(
                "Limit order requires positive price".into(),
            ));
        }
        adapter.place_limit_order(intent)
    }
}

/// Strategy 3: Limit-Then-Market Execution Algorithm.
///
/// Submits a resting limit order first. If not fully filled within `max_wait_ms`,
/// it cancels the uncrossed remainder and routes the residual balance via Market Order
/// with a deterministic fallback client order ID.
pub struct LimitThenMarketExecutor {
    pub max_wait_ms: u64,
    pub fallback_to_market: bool,
}

impl LimitThenMarketExecutor {
    pub fn new(max_wait_ms: u64, fallback_to_market: bool) -> Self {
        Self {
            max_wait_ms,
            fallback_to_market,
        }
    }
}

impl OrderExecutionStrategy for LimitThenMarketExecutor {
    fn execute<A: ExchangeOrderAdapter>(
        &self,
        adapter: &A,
        intent: &OrderIntent,
    ) -> Result<FillSnapshot, DomainError> {
        // If price is unspecified or 0, fallback immediately to market
        if intent.price.is_none() || intent.price.unwrap_or(0.0) <= 0.0 {
            return adapter.place_market_order(intent);
        }

        // 1. Submit resting limit order
        let initial_fill = adapter.place_limit_order(intent)?;
        if initial_fill.is_complete(intent.quantity) {
            return Ok(initial_fill);
        }

        // 2. Wait up to timeout window
        let fill = adapter.wait_for_fill(intent, &initial_fill.order_id, self.max_wait_ms)?;
        if fill.is_complete(intent.quantity) || !self.fallback_to_market {
            return Ok(fill);
        }

        // 3. Cancel residual uncrossed balance
        let _ = adapter.cancel_order(&intent.symbol, &initial_fill.order_id);
        let confirmed = adapter.wait_for_fill(intent, &initial_fill.order_id, self.max_wait_ms)?;

        let limit_filled = confirmed.filled_qty.max(fill.filled_qty);
        let remaining_qty = (intent.quantity - limit_filled).max(0.0);

        if remaining_qty <= 0.000001 {
            return Ok(confirmed);
        }

        // 4. Route residual balance to Market Order
        let fallback_id = intent
            .fallback_client_order_id
            .clone()
            .unwrap_or_else(|| format!("{}_mkt", intent.client_order_id));

        let mut market_intent = intent.clone();
        market_intent.quantity = remaining_qty;
        market_intent.price = None;
        market_intent.client_order_id = fallback_id;

        adapter.place_market_order(&market_intent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockAdapter {
        market_call_count: AtomicUsize,
        limit_call_count: AtomicUsize,
        cancel_call_count: AtomicUsize,
        mock_limit_fill_qty: f64,
    }

    impl ExchangeOrderAdapter for MockAdapter {
        fn venue_id(&self) -> &'static str {
            "mock_venue"
        }

        fn place_market_order(&self, intent: &OrderIntent) -> Result<FillSnapshot, DomainError> {
            self.market_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(FillSnapshot {
                order_id: format!("{}_mkt_fill", intent.client_order_id),
                filled_qty: intent.quantity,
                avg_price: 150.0,
                status: ExecutionStatus::Filled,
                fee_quote: 0.15,
            })
        }

        fn place_limit_order(&self, intent: &OrderIntent) -> Result<FillSnapshot, DomainError> {
            self.limit_call_count.fetch_add(1, Ordering::SeqCst);
            let status = if self.mock_limit_fill_qty >= intent.quantity {
                ExecutionStatus::Filled
            } else if self.mock_limit_fill_qty > 0.0 {
                ExecutionStatus::Partial
            } else {
                ExecutionStatus::Submitted
            };
            Ok(FillSnapshot {
                order_id: format!("{}_lim_fill", intent.client_order_id),
                filled_qty: self.mock_limit_fill_qty,
                avg_price: intent.price.unwrap_or(100.0),
                status,
                fee_quote: 0.05,
            })
        }

        fn cancel_order(&self, _symbol: &str, _order_id: &str) -> Result<(), DomainError> {
            self.cancel_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn wait_for_fill(
            &self,
            intent: &OrderIntent,
            _order_id: &str,
            _timeout: u64,
        ) -> Result<FillSnapshot, DomainError> {
            let status = if self.mock_limit_fill_qty >= intent.quantity {
                ExecutionStatus::Filled
            } else if self.mock_limit_fill_qty > 0.0 {
                ExecutionStatus::Partial
            } else {
                ExecutionStatus::Submitted
            };
            Ok(FillSnapshot {
                order_id: format!("{}_lim_fill", intent.client_order_id),
                filled_qty: self.mock_limit_fill_qty,
                avg_price: intent.price.unwrap_or(100.0),
                status,
                fee_quote: 0.05,
            })
        }

        fn query_position(&self, symbol: &str) -> Result<Option<PositionSnapshot>, DomainError> {
            Ok(Some(PositionSnapshot {
                symbol: symbol.to_string(),
                side: OrderSide::Buy,
                base_qty: 10.0,
                avg_price: 100.0,
            }))
        }
    }

    #[test]
    fn test_limit_then_market_full_limit_fill() {
        let adapter = MockAdapter {
            market_call_count: AtomicUsize::new(0),
            limit_call_count: AtomicUsize::new(0),
            cancel_call_count: AtomicUsize::new(0),
            mock_limit_fill_qty: 10.0,
        };

        let intent = OrderIntent::new("BTC/USDT", OrderSide::Buy, 10.0, "ord_01")
            .unwrap()
            .with_limit_price(50_000.0)
            .unwrap();

        let executor = LimitThenMarketExecutor::new(1000, true);
        let result = executor.execute(&adapter, &intent).expect("execute");

        assert_eq!(result.status, ExecutionStatus::Filled);
        assert_eq!(result.filled_qty, 10.0);
        assert_eq!(adapter.limit_call_count.load(Ordering::SeqCst), 1);
        assert_eq!(adapter.cancel_call_count.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.market_call_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_limit_then_market_partial_fill_fallback() {
        let adapter = MockAdapter {
            market_call_count: AtomicUsize::new(0),
            limit_call_count: AtomicUsize::new(0),
            cancel_call_count: AtomicUsize::new(0),
            mock_limit_fill_qty: 4.0, // Partial fill: 4 out of 10
        };

        let intent = OrderIntent::new("ETH/USDT", OrderSide::Buy, 10.0, "ord_02")
            .unwrap()
            .with_limit_price(3_000.0)
            .unwrap()
            .with_fallback_id("ord_02_custom_mkt");

        let executor = LimitThenMarketExecutor::new(1000, true);
        let result = executor.execute(&adapter, &intent).expect("execute");

        // The residual 6.0 units must be filled via market
        assert_eq!(result.status, ExecutionStatus::Filled);
        assert_eq!(result.filled_qty, 6.0);
        assert_eq!(adapter.limit_call_count.load(Ordering::SeqCst), 1);
        assert_eq!(adapter.cancel_call_count.load(Ordering::SeqCst), 1);
        assert_eq!(adapter.market_call_count.load(Ordering::SeqCst), 1);
    }
}
