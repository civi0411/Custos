//! Quantitative trading application handlers organized by domain submodules:
//! - backtest: backtesting simulation, tearsheet persistence, PBO evaluation, grid search, indicators
//! - orders: order ticket creation, approval governance, kill switch emergency stop, orderbook depth
//! - market: market data ingestion/query, factor discovery, pre-trade risk gates, post-trade reviews

pub mod backtest;
pub mod dto;
pub mod market;
pub mod orders;
pub mod services;

use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant(&self, req: ApiRequest) -> ApiResponse {
        let is_read_only = matches!(
            req.method.as_str(),
            METHOD_QUANT_TEARSHEET_GET
                | METHOD_QUANT_ORDERS_LIST
                | METHOD_QUANT_MARKET_BARS_GET
                | METHOD_QUANT_ORDERBOOK_GET
        );
        if !is_read_only {
            if let Err(denied) = self.require_pack_enabled("quant", &req.id) {
                return denied;
            }
        }
        match req.method.as_str() {
            METHOD_QUANT_BACKTEST_SUBMIT
            | METHOD_QUANT_TEARSHEET_GET
            | METHOD_QUANT_PBO_COMPUTE
            | METHOD_QUANT_GRID_EVALUATE
            | METHOD_QUANT_INDICATORS_COMPUTE => self.handle_quant_backtest(req).await,

            METHOD_QUANT_ORDERS_CREATE
            | METHOD_QUANT_ORDERS_LIST
            | METHOD_QUANT_ORDERS_APPROVE
            | METHOD_QUANT_KILL_SWITCH
            | METHOD_QUANT_ORDERBOOK_GET => self.handle_quant_orders(req).await,

            METHOD_QUANT_FACTORS_DISCOVER
            | METHOD_QUANT_MARKET_BARS_INGEST
            | METHOD_QUANT_MARKET_BARS_GET
            | METHOD_QUANT_GATE_EVALUATE
            | METHOD_QUANT_REVIEW_COMPUTE => self.handle_quant_market(req).await,

            _ => ApiResponse::error(req.id, format!("Unknown quant method: {}", req.method)),
        }
    }
}
