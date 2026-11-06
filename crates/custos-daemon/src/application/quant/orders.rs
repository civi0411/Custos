//! Order execution, ticket creation, approval workflow, kill switch and orderbook depth (Transport Adapter).

use crate::api::*;
use crate::application::quant::dto::*;
use crate::application::quant::services::OrderExecutionService;

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant_orders(&self, req: ApiRequest) -> ApiResponse {
        let store = match self.sqlite_store.as_ref() {
            Some(s) => s,
            None => return ApiResponse::error(req.id, "SqliteTaskStore not configured on daemon"),
        };

        match req.method.as_str() {
            METHOD_QUANT_ORDERS_CREATE => {
                let cmd: CreateOrdersRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid create orders payload: {e}"),
                        )
                    }
                };
                match OrderExecutionService::create_orders(cmd, store.quant()).await {
                    Ok(created) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "created_orders": created }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_ORDERS_LIST => {
                let cmd: ListOrdersRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid list orders payload: {e}"),
                        )
                    }
                };
                match OrderExecutionService::list_orders(cmd, store.quant()).await {
                    Ok(orders) => {
                        ApiResponse::success(req.id, serde_json::json!({ "orders": orders }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to list orders: {e}")),
                }
            }

            METHOD_QUANT_ORDERS_APPROVE => {
                let cmd: ApproveOrderRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid approve order payload: {e}"),
                        )
                    }
                };
                let outbox = self.outbox_repo();
                match OrderExecutionService::approve_order(
                    cmd,
                    store.quant(),
                    outbox.map(|o| o as &dyn custos_core::contracts::storage::OutboxPort),
                )
                .await
                {
                    Ok(result) => ApiResponse::success(req.id, result),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_KILL_SWITCH => {
                let cmd: KillSwitchRequest =
                    serde_json::from_value(req.params).unwrap_or(KillSwitchRequest {
                        reason: "Manual Emergency Kill-Switch Activated".into(),
                    });
                let outbox = self.outbox_repo();
                match OrderExecutionService::kill_switch(
                    cmd,
                    store.quant(),
                    outbox.map(|o| o as &dyn custos_core::contracts::storage::OutboxPort),
                )
                .await
                {
                    Ok(result) => ApiResponse::success(req.id, result),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_ORDERBOOK_GET => {
                let cmd: GetOrderbookRequest =
                    serde_json::from_value(req.params).unwrap_or(GetOrderbookRequest {
                        symbol: "BTC/USDT".into(),
                        depth: 5,
                    });
                match OrderExecutionService::get_orderbook(cmd, Some(store.quant())).await {
                    Ok(snapshot) => ApiResponse::success(
                        req.id,
                        serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            _ => ApiResponse::error(
                req.id,
                format!("Unknown quant orders method: {}", req.method),
            ),
        }
    }
}
