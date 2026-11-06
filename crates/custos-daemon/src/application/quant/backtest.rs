//! Backtesting, tearsheets, simulation engines, PBO and technical indicators (Transport Adapter).

use crate::api::*;
use crate::application::quant::dto::*;
use crate::application::quant::services::BacktestService;

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant_backtest(&self, req: ApiRequest) -> ApiResponse {
        let store = match self.sqlite_store.as_ref() {
            Some(s) => s,
            None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
        };

        match req.method.as_str() {
            METHOD_QUANT_BACKTEST_SUBMIT => {
                let cmd: SubmitBacktestRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid backtest submit payload: {e}"),
                        )
                    }
                };
                match BacktestService::submit_backtest(cmd, store.quant()).await {
                    Ok(sheet) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&sheet).unwrap_or_default(),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_TEARSHEET_GET => {
                let cmd: GetTearsheetRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Missing 'strategy_id' parameter: {e}"),
                        )
                    }
                };
                match BacktestService::get_tearsheet(&cmd.strategy_id, store.quant()).await {
                    Ok(Some(sheet)) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&sheet).unwrap_or_default(),
                    ),
                    Ok(None) => ApiResponse::error(
                        req.id,
                        format!("No tearsheet found for strategy '{}'", cmd.strategy_id),
                    ),
                    Err(e) => ApiResponse::error(req.id, format!("Error reading tearsheet: {e}")),
                }
            }

            METHOD_QUANT_GRID_EVALUATE => {
                let cmd: GridEvaluateRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid grid payload: {e}"))
                    }
                };
                match BacktestService::evaluate_grid(cmd) {
                    Ok(res) => ApiResponse::success(req.id, res),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_INDICATORS_COMPUTE => {
                let cmd: ComputeIndicatorsRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid compute indicators payload: {e}"),
                        )
                    }
                };
                match BacktestService::compute_indicators(cmd, store.quant()).await {
                    Ok(res) => ApiResponse::success(req.id, res),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_PBO_COMPUTE => {
                let cmd: ComputePboRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid PBO payload: {e}"))
                    }
                };
                match BacktestService::compute_pbo(cmd) {
                    Ok(res) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&res).unwrap_or(serde_json::Value::Null),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            _ => ApiResponse::error(
                req.id,
                format!("Unknown quant backtest method: {}", req.method),
            ),
        }
    }
}
