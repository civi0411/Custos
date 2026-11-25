//! Order execution, ticket creation, approval workflow, position closing (Exit Immunity),
//! kill switch and orderbook depth (Transport Adapter).

use crate::api::*;
use custos_contracts::{ActionIntent, RiskLevel};
use custos_engine::contracts::kernel::{KernelPort, TrustedKernel};
use custos_workbench_quant::services::{OrderExecutionService, *};

impl LocalApiDispatcher {
    async fn mint_operator_permit(
        &self,
        task_id: &str,
        action_name: &str,
        target: &str,
        payload: serde_json::Value,
    ) -> Result<String, String> {
        let Some(store) = self.kernel.sqlite_store.as_ref() else {
            return Ok(custos_contracts::new_id("pmt_op"));
        };
        let kernel = TrustedKernel::new(store.clone());
        let intent = ActionIntent::new(
            custos_contracts::new_id("act"),
            action_name.to_string(),
            target.to_string(),
            payload,
            RiskLevel::Low, // Human operator explicitly triggered sign-off on Desktop UI
        )
        .with_task_id(task_id.to_string());
        let permit = kernel
            .request_permit(task_id, &intent, "human_operator")
            .await
            .map_err(|e| format!("Kernel permit issuance failed: {e}"))?;
        let consumed = kernel
            .consume_permit(&permit.id, &intent)
            .await
            .map_err(|e| format!("Kernel permit consumption failed: {e}"))?;
        Ok(consumed.id)
    }

    pub(crate) async fn handle_quant_orders(&self, req: ApiRequest) -> ApiResponse {
        let Some(quant_arc) = self.quant_port() else {
            return ApiResponse::error(req.id, "Quant workbench is not enabled on daemon");
        };
        let quant: &dyn custos_workbench_quant::QuantPort = quant_arc.as_ref();

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
                match OrderExecutionService::create_orders(cmd, quant).await {
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
                match OrderExecutionService::list_orders(cmd, quant).await {
                    Ok(orders) => {
                        ApiResponse::success(req.id, serde_json::json!({ "orders": orders }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to list orders: {e}")),
                }
            }

            METHOD_QUANT_ORDERS_APPROVE => {
                let mut cmd: ApproveOrderRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid approve order payload: {e}"),
                        )
                    }
                };
                if cmd.permit_id == "auto_issue" {
                    match self
                        .mint_operator_permit(
                            &cmd.task_id,
                            "quant_order_approve",
                            &cmd.order_id,
                            serde_json::json!({ "order_id": cmd.order_id }),
                        )
                        .await
                    {
                        Ok(real_permit) => cmd.permit_id = real_permit,
                        Err(e) => return ApiResponse::error(req.id, e),
                    }
                }
                let outbox = self.outbox_repo();
                match OrderExecutionService::approve_order(
                    cmd,
                    quant,
                    outbox.map(|o| o as &dyn custos_engine::contracts::storage::OutboxPort),
                )
                .await
                {
                    Ok(result) => ApiResponse::success(req.id, result),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }

            METHOD_QUANT_ORDERS_CLOSE_POSITION => {
                let mut cmd: ClosePositionRequest = match serde_json::from_value(req.params) {
                    Ok(c) => c,
                    Err(e) => {
                        return ApiResponse::error(
                            req.id,
                            format!("Invalid close position payload: {e}"),
                        )
                    }
                };
                if cmd.permit_id.as_deref() == Some("auto_issue") {
                    match self
                        .mint_operator_permit(
                            &cmd.task_id,
                            "quant_position_close",
                            &cmd.symbol,
                            serde_json::json!({
                                "symbol": cmd.symbol,
                                "quantity": cmd.quantity,
                                "direction": cmd.direction
                            }),
                        )
                        .await
                    {
                        Ok(real_permit) => cmd.permit_id = Some(real_permit),
                        Err(e) => return ApiResponse::error(req.id, e),
                    }
                }
                let outbox = self.outbox_repo();
                match OrderExecutionService::close_position(
                    cmd,
                    quant,
                    outbox.map(|o| o as &dyn custos_engine::contracts::storage::OutboxPort),
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
                    quant,
                    outbox.map(|o| o as &dyn custos_engine::contracts::storage::OutboxPort),
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
                match OrderExecutionService::get_orderbook(cmd, Some(quant)).await {
                    Ok(snapshot) => {
                        let val = serde_json::to_value(&snapshot).unwrap_or_default();
                        ApiResponse::success(req.id, val)
                    }
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
