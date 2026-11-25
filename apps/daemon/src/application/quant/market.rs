//! Market data ingestion, factor discovery, strategies, positions, fills, evidence quality,
//! pre-trade risk gates, and post-trade reviews (Transport Adapter).

use crate::api::*;
use custos_workbench_quant::services::{MarketService, *};

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant_market(&self, req: ApiRequest) -> ApiResponse {
        let Some(quant_arc) = self.quant_port() else {
            return ApiResponse::error(req.id, "Quant workbench is not enabled on daemon");
        };
        let quant: &dyn custos_workbench_quant::QuantPort = quant_arc.as_ref();

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
                match MarketService::discover_factors(cmd, quant).await {
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
                match MarketService::ingest_market_bars(cmd, quant).await {
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
                match MarketService::get_market_bars(cmd, quant).await {
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

            METHOD_QUANT_STRATEGIES_LIST => match MarketService::list_strategies(quant).await {
                Ok(strategies) => {
                    ApiResponse::success(req.id, serde_json::json!({ "strategies": strategies }))
                }
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },

            METHOD_QUANT_STRATEGIES_SAVE => {
                let cmd: SaveStrategyRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid save strategy payload: {e}"),
                        )
                    }
                };
                match MarketService::save_strategy(cmd, quant).await {
                    Ok(strategy) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&strategy).unwrap_or(serde_json::Value::Null),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_POSITIONS_LIST => {
                let cmd: ListPositionsRequest = serde_json::from_value(req.params)
                    .unwrap_or(ListPositionsRequest { strategy_id: None });
                match MarketService::list_positions(cmd, quant).await {
                    Ok(positions) => {
                        ApiResponse::success(req.id, serde_json::json!({ "positions": positions }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_FILLS_LIST => {
                let cmd: ListFillsRequest =
                    serde_json::from_value(req.params).unwrap_or(ListFillsRequest {
                        strategy_id: None,
                        symbol: None,
                        limit: 500,
                    });
                match MarketService::list_fills(cmd, quant).await {
                    Ok(fills) => {
                        ApiResponse::success(req.id, serde_json::json!({ "fills": fills }))
                    }
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_EVIDENCE_ASSESS => {
                let cmd: AssessEvidenceRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid evidence assess payload: {e}"),
                        )
                    }
                };
                match MarketService::assess_evidence(cmd, quant).await {
                    Ok(summary) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&summary).unwrap_or(serde_json::Value::Null),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
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
                match MarketService::evaluate_gate_with_store(cmd, quant).await {
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
                match MarketService::compute_review_with_store(cmd, quant).await {
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
