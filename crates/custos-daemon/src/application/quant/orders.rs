//! Order execution, ticket creation, approval workflow, kill switch and orderbook depth.

use crate::api::*;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus, QuantPort};
use custos_domain::quant::{
    OrderBookSnapshot, OrderDirection, OrderTicket, OrderTicketStatus, OrderType,
};

impl LocalApiDispatcher {
    pub(crate) async fn handle_quant_orders(&self, req: ApiRequest) -> ApiResponse {
        match req.method.as_str() {
            METHOD_QUANT_ORDERS_CREATE => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                let strategy_id = match req.params.get("strategy_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'strategy_id' parameter"),
                };
                let task_id = match req.params.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'task_id' parameter"),
                };
                let orders_arr = match req.params.get("orders").and_then(|v| v.as_array()) {
                    Some(arr) => arr,
                    None => return ApiResponse::error(req.id, "Missing 'orders' array parameter"),
                };

                let mut created = Vec::new();
                for ord in orders_arr {
                    let symbol = match ord.get("symbol").and_then(|v| v.as_str()) {
                        Some(s) => s,
                        None => continue,
                    };
                    let dir_str = ord
                        .get("direction")
                        .and_then(|v| v.as_str())
                        .unwrap_or("buy");
                    let direction = match dir_str {
                        "sell" => OrderDirection::Sell,
                        "sell_short" => OrderDirection::SellShort,
                        "buy_to_cover" => OrderDirection::BuyToCover,
                        _ => OrderDirection::Buy,
                    };
                    let qty = ord
                        .get("quantity")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(100.0);
                    let weight = ord
                        .get("target_weight")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.05);

                    let mut ticket =
                        match OrderTicket::new(strategy_id, symbol, direction, qty, weight) {
                            Ok(t) => t,
                            Err(e) => {
                                return ApiResponse::error(req.id, format!("Invalid order: {e}"))
                            }
                        };

                    if let Some(limit) = ord.get("limit_price").and_then(|v| v.as_f64()) {
                        ticket = ticket.with_limit_price(limit);
                    }
                    if let Some(slippage) =
                        ord.get("estimated_slippage_bps").and_then(|v| v.as_f64())
                    {
                        ticket = ticket.with_algorithmic_execution(OrderType::Twap, slippage);
                    }

                    if let Err(e) = store.quant().save_order_ticket(&ticket, task_id).await {
                        return ApiResponse::error(
                            req.id,
                            format!("Failed to save order ticket: {e}"),
                        );
                    }
                    created.push(ticket);
                }

                ApiResponse::success(req.id, serde_json::json!({ "created_orders": created }))
            }

            METHOD_QUANT_ORDERS_LIST => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                let strategy_id = req.params.get("strategy_id").and_then(|v| v.as_str());
                let task_id = req.params.get("task_id").and_then(|v| v.as_str());

                match store.quant().list_order_tickets(strategy_id, task_id).await {
                    Ok(orders) => {
                        ApiResponse::success(req.id, serde_json::json!({ "orders": orders }))
                    }
                    Err(e) => ApiResponse::error(req.id, format!("Failed to list orders: {e}")),
                }
            }

            METHOD_QUANT_ORDERS_APPROVE => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                let order_id = match req.params.get("order_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing 'order_id' parameter"),
                };
                let permit_id = match req.params.get("permit_id").and_then(|v| v.as_str()) {
                    Some(id) if !id.trim().is_empty() => id.to_string(),
                    _ => {
                        return ApiResponse::error(
                            req.id,
                            "Missing or empty 'permit_id' parameter for order approval",
                        )
                    }
                };
                let task_id = req
                    .params
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("quant_task");

                let mut ticket = match store.quant().get_order_ticket(order_id).await {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        return ApiResponse::error(req.id, format!("Order '{order_id}' not found"))
                    }
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Failed to get order: {e}"))
                    }
                };

                if let Err(e) = ticket.approve_with_permit(permit_id.clone()) {
                    return ApiResponse::error(
                        req.id,
                        format!("Approval state transition failed: {e}"),
                    );
                }

                if let Err(e) = store.quant().update_order_ticket(&ticket).await {
                    return ApiResponse::error(
                        req.id,
                        format!("Failed to update order status: {e}"),
                    );
                }

                // Bridge to OutboxActionItem for governed delivery
                let outbox_item = ticket.to_outbox_action_item(task_id);
                if let Some(outbox) = self.outbox_repo() {
                    let outbox_entry = OutboxEntry {
                        id: outbox_item.id.clone(),
                        task_id: task_id.to_string(),
                        action_id: outbox_item.action_type.clone(),
                        permit_id: permit_id.clone(),
                        argument_digest: outbox_item.payload_digest.clone(),
                        idempotency_key: None,
                        status: OutboxStatus::Approved,
                        created_at: chrono::Utc::now(),
                        receipt: None,
                        recipient: Some(outbox_item.recipient.clone()),
                        payload_json: outbox_item.payload_json.clone(),
                        error_message: None,
                    };
                    let _ = outbox.enqueue(outbox_entry).await;
                }

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "approved_ticket": ticket,
                        "status": "approved",
                        "permit_id": permit_id,
                        "outbox_action_id": outbox_item.id,
                    }),
                )
            }

            METHOD_QUANT_KILL_SWITCH => {
                let store = match self.sqlite_store.as_ref() {
                    Some(s) => s,
                    None => {
                        return ApiResponse::error(
                            req.id,
                            "SqliteTaskStore not configured on daemon",
                        )
                    }
                };
                let reason = req
                    .params
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Manual Emergency Kill-Switch Activated");

                // Cancel all draft or awaiting orders
                let all_orders = store
                    .quant()
                    .list_order_tickets(None, None)
                    .await
                    .unwrap_or_default();
                let mut cancelled_count = 0;
                for mut ticket in all_orders {
                    if ticket.status == OrderTicketStatus::Draft
                        || ticket.status == OrderTicketStatus::AwaitingPermit
                        || ticket.status == OrderTicketStatus::Approved
                    {
                        let _ = ticket.cancel();
                        let _ = store.quant().update_order_ticket(&ticket).await;
                        cancelled_count += 1;
                    }
                }

                // Record emergency event in outbox
                if let Some(outbox) = self.outbox_repo() {
                    let entry = OutboxEntry {
                        id: custos_domain::new_id("kill_switch"),
                        task_id: "emergency_stop".into(),
                        action_id: "quant_emergency_stop".into(),
                        permit_id: "emergency_permit".into(),
                        argument_digest: custos_domain::digest(reason.as_bytes()),
                        idempotency_key: None,
                        status: OutboxStatus::Receipted,
                        created_at: chrono::Utc::now(),
                        receipt: None,
                        recipient: Some("broker_execution_gateway".into()),
                        payload_json: Some(
                            serde_json::json!({ "reason": reason, "cancelled_orders": cancelled_count })
                                .to_string(),
                        ),
                        error_message: None,
                    };
                    let _ = outbox.enqueue(entry).await;
                }

                ApiResponse::success(
                    req.id,
                    serde_json::json!({
                        "status": "emergency_stop_executed",
                        "reason": reason,
                        "cancelled_orders_count": cancelled_count,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                )
            }

            METHOD_QUANT_ORDERBOOK_GET => {
                let symbol = req
                    .params
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("BTC/USDT");
                let depth_levels = req
                    .params
                    .get("depth")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5)
                    .min(20) as usize;

                let store = self.sqlite_store.as_ref();
                let last_bar_close = if let Some(s) = store {
                    match s.quant().get_market_bars(symbol, "1m", 1).await {
                        Ok(bars) if !bars.is_empty() => Some(bars[0].close),
                        _ => match s.quant().get_market_bars(symbol, "1d", 1).await {
                            Ok(bars) if !bars.is_empty() => Some(bars[0].close),
                            _ => None,
                        },
                    }
                } else {
                    None
                };

                let close_price = last_bar_close.unwrap_or(if symbol.starts_with("BTC") {
                    68000.0
                } else {
                    100.0
                });
                let snapshot = OrderBookSnapshot::synthesize(symbol, close_price, depth_levels);
                ApiResponse::success(
                    req.id,
                    serde_json::to_value(&snapshot).unwrap_or(serde_json::Value::Null),
                )
            }

            _ => ApiResponse::error(
                req.id,
                format!("Unknown quant orders method: {}", req.method),
            ),
        }
    }
}
