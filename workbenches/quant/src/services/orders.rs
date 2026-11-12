//! Application Service orchestrating Order tickets, approval outbox governance, and emergency stops.

use super::dto::*;
use custos_core::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};
use crate::{
    OrderBookSnapshot, OrderDirection, OrderTicket, OrderTicketStatus, OrderType, QuantPort,
};
use custos_domain::DomainError;

pub struct OrderExecutionService;

impl OrderExecutionService {
    pub async fn create_orders(
        req: CreateOrdersRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<Vec<OrderTicket>, DomainError> {
        if req.strategy_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'strategy_id' parameter".into(),
            ));
        }
        if req.task_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'task_id' parameter".into(),
            ));
        }

        let mut created = Vec::with_capacity(req.orders.len());
        for ord in req.orders {
            let direction = match ord.direction.to_lowercase().as_str() {
                "sell" => OrderDirection::Sell,
                "sell_short" => OrderDirection::SellShort,
                "buy_to_cover" => OrderDirection::BuyToCover,
                _ => OrderDirection::Buy,
            };

            let mut ticket = OrderTicket::new(
                &req.strategy_id,
                &ord.symbol,
                direction,
                ord.quantity,
                ord.target_weight,
            )
            .map_err(|e| DomainError::Validation(format!("Invalid order: {e}")))?;

            if let Some(limit) = ord.limit_price {
                ticket = ticket.with_limit_price(limit);
            }
            if let Some(slippage) = ord.estimated_slippage_bps {
                ticket = ticket.with_algorithmic_execution(OrderType::Twap, slippage);
            }

            quant_port.save_order_ticket(&ticket, &req.task_id).await?;
            created.push(ticket);
        }

        Ok(created)
    }

    pub async fn list_orders(
        req: ListOrdersRequest,
        quant_port: &dyn QuantPort,
    ) -> Result<Vec<OrderTicket>, DomainError> {
        quant_port
            .list_order_tickets(req.strategy_id.as_deref(), req.task_id.as_deref())
            .await
    }

    pub async fn approve_order(
        req: ApproveOrderRequest,
        quant_port: &dyn QuantPort,
        outbox_port: Option<&dyn OutboxPort>,
    ) -> Result<serde_json::Value, DomainError> {
        if req.permit_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Missing or empty 'permit_id' parameter for order approval".into(),
            ));
        }

        let mut ticket = quant_port
            .get_order_ticket(&req.order_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                kind: "Order".into(),
                id: req.order_id.clone(),
            })?;

        ticket
            .approve_with_permit(req.permit_id.clone())
            .map_err(|e| {
                DomainError::Validation(format!("Approval state transition failed: {e}"))
            })?;

        quant_port.update_order_ticket(&ticket).await?;

        // Bridge to OutboxActionItem for governed delivery
        let outbox_item = ticket.to_outbox_action_item(&req.task_id);
        if let Some(outbox) = outbox_port {
            let outbox_entry = OutboxEntry {
                id: outbox_item.id.clone(),
                task_id: req.task_id,
                action_id: outbox_item.action_type.clone(),
                permit_id: req.permit_id.clone(),
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

        Ok(serde_json::json!({
            "approved_ticket": ticket,
            "status": "approved",
            "permit_id": req.permit_id,
            "outbox_action_id": outbox_item.id,
        }))
    }

    pub async fn kill_switch(
        req: KillSwitchRequest,
        quant_port: &dyn QuantPort,
        outbox_port: Option<&dyn OutboxPort>,
    ) -> Result<serde_json::Value, DomainError> {
        // Cancel all draft or awaiting orders
        let all_orders = quant_port
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
                let _ = quant_port.update_order_ticket(&ticket).await;
                cancelled_count += 1;
            }
        }

        // Record emergency event in outbox
        if let Some(outbox) = outbox_port {
            let entry = OutboxEntry {
                id: custos_domain::new_id("kill_switch"),
                task_id: "emergency_stop".into(),
                action_id: "quant_emergency_stop".into(),
                permit_id: "emergency_permit".into(),
                argument_digest: custos_domain::digest(req.reason.as_bytes()),
                idempotency_key: None,
                status: OutboxStatus::Receipted,
                created_at: chrono::Utc::now(),
                receipt: None,
                recipient: Some("broker_execution_gateway".into()),
                payload_json: Some(
                    serde_json::json!({ "reason": req.reason, "cancelled_orders": cancelled_count })
                        .to_string(),
                ),
                error_message: None,
            };
            let _ = outbox.enqueue(entry).await;
        }

        Ok(serde_json::json!({
            "status": "emergency_stop_executed",
            "reason": req.reason,
            "cancelled_orders_count": cancelled_count,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }

    pub async fn get_orderbook(
        req: GetOrderbookRequest,
        quant_port: Option<&dyn QuantPort>,
    ) -> Result<OrderBookSnapshot, DomainError> {
        let depth_levels = req.depth.clamp(1, 20);

        let last_bar_close = if let Some(qp) = quant_port {
            match qp.get_market_bars(&req.symbol, "1m", 1).await {
                Ok(bars) if !bars.is_empty() => Some(bars[0].close),
                _ => match qp.get_market_bars(&req.symbol, "1d", 1).await {
                    Ok(bars) if !bars.is_empty() => Some(bars[0].close),
                    _ => None,
                },
            }
        } else {
            None
        };

        let close_price = last_bar_close.ok_or_else(|| {
            DomainError::Validation(format!(
                "No market data bars ingested for '{}'; ingest OHLCV bars before requesting an orderbook",
                req.symbol
            ))
        })?;

        OrderBookSnapshot::synthesize(&req.symbol, close_price, depth_levels)
    }
}
