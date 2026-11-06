//! Market data ingestion, factor discovery, pre-trade risk gates and post-trade reviews (Transport Adapter).

use crate::api::*;
use crate::application::quant::dto::*;
use crate::application::quant::services::MarketService;

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant_market(&self, req: ApiRequest) -> ApiResponse {
        let store = match self.sqlite_store.as_ref() {
            Some(s) => s,
            None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
        };

        match req.method.as_str() {
            METHOD_QUANT_FACTORS_DISCOVER => {
                let cmd: DiscoverFactorsRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid factors discover payload: {e}"),
                        )
                    }
                };
                match MarketService::discover_factors(cmd, store.quant()).await {
                    Ok(res) => ApiResponse::success(req.id, res),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_MARKET_BARS_INGEST => {
                let cmd: IngestMarketBarsRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid market bars ingest payload: {e}"),
                        )
                    }
                };
                let symbol = cmd.symbol.clone();
                let timeframe = cmd.timeframe.clone();
                match MarketService::ingest_market_bars(cmd, store.quant()).await {
                    Ok(count) => ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "symbol": symbol,
                            "timeframe": timeframe,
                            "stored_bars": count,
                        }),
                    ),
                    Err(e) => {
                        ApiResponse::error(req.id, format!("Failed to save market bars: {e}"))
                    }
                }
            }

            METHOD_QUANT_MARKET_BARS_GET => {
                let cmd: GetMarketBarsRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid market bars get payload: {e}"),
                        )
                    }
                };
                let symbol = cmd.symbol.clone();
                let timeframe = cmd.timeframe.clone();
                match MarketService::get_market_bars(cmd, store.quant()).await {
                    Ok(bars) => ApiResponse::success(
                        req.id,
                        serde_json::json!({
                            "symbol": symbol,
                            "timeframe": timeframe,
                            "bars": bars,
                            "count": bars.len(),
                        }),
                    ),
                    Err(e) => ApiResponse::error(req.id, format!("Failed to get market bars: {e}")),
                }
            }

            METHOD_QUANT_GATE_EVALUATE => {
                let cmd: EvaluateGateRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid gate evaluate payload: {e}"),
                        )
                    }
                };
                match MarketService::evaluate_gate(cmd) {
                    Ok(res) => ApiResponse::success(req.id, res),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_REVIEW_COMPUTE => {
                let cmd: ComputeReviewRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid review compute payload: {e}"),
                        )
                    }
                };
                match MarketService::compute_review(cmd) {
                    Ok(report) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&report).unwrap_or(serde_json::Value::Null),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            _ => ApiResponse::error(
                req.id,
                format!("Unknown quant market method: {}", req.method),
            ),
        }
    }
}
