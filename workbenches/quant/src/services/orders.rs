//! Application Service orchestrating Order tickets, permit-bound outbox governance,
//! deterministic paper fill settlement, position closing (Exit Immunity), and emergency stops.

use super::dto::*;
use crate::{
    CloseReason, DecisionContext, OrderBookSnapshot, OrderDirection, OrderTicket,
    OrderTicketStatus, OrderType, PreTradeDecisionGate, QuantPort, QuantPositionRecord,
    TradeFillRecord,
};
use custos_contracts::{digest, new_id, Assurance, DomainError, ExecutionReceipt, ReceiptStatus};
use custos_engine::contracts::storage::{OutboxEntry, OutboxPort, OutboxStatus};

pub struct OrderExecutionService;

async fn settle_paper_fill(
    ticket: &mut OrderTicket,
    task_id: &str,
    explicit_price: Option<f64>,
    close_reason: Option<CloseReason>,
    quant_port: &dyn QuantPort,
) -> Result<TradeFillRecord, DomainError> {
    let fill_price = match explicit_price.or(ticket.limit_price) {
        Some(p) if p > 0.0 => p,
        _ => {
            let bars = quant_port
                .get_market_bars(&ticket.symbol, "1d", 1)
                .await
                .unwrap_or_default();
            bars.last()
                .map(|b| b.close)
                .filter(|&p| p > 0.0)
                .ok_or_else(|| {
                    DomainError::Validation(format!(
                        "Cannot settle paper fill for '{}': no limit_price or market bars available",
                        ticket.symbol
                    ))
                })?
        }
    };

    let slippage_sign = match ticket.direction {
        OrderDirection::Buy | OrderDirection::BuyToCover => 1.0,
        OrderDirection::Sell | OrderDirection::SellShort => -1.0,
    };
    let exec_price = fill_price * (1.0 + slippage_sign * ticket.estimated_slippage_bps * 1e-4);
    let commission = (exec_price * ticket.quantity).abs() * 0.0005;
    let now = chrono::Utc::now().timestamp();

    let fill = TradeFillRecord {
        fill_id: new_id("fill"),
        order_id: ticket.id.clone(),
        symbol: ticket.symbol.clone(),
        direction: ticket.direction,
        price: exec_price,
        quantity: ticket.quantity,
        commission,
        timestamp: now,
        exit_reason: close_reason.map(|r| r.as_str().to_string()),
    };

    quant_port
        .save_fill(&fill, &ticket.strategy_id, task_id, "paper")
        .await?;

    // Reconcile persisted position state in SQLite
    let existing_positions = quant_port.list_positions(Some(&ticket.strategy_id)).await?;
    let (pos_dir, is_closing) = match ticket.direction {
        OrderDirection::Buy => ("long", false),
        OrderDirection::Sell => ("long", true),
        OrderDirection::SellShort => ("short", false),
        OrderDirection::BuyToCover => ("short", true),
    };

    let strat_market = quant_port
        .get_strategy(&ticket.strategy_id)
        .await
        .ok()
        .flatten()
        .map(|s| s.market)
        .unwrap_or_else(|| "USStock".to_string());

    if let Some(mut pos) = existing_positions
        .into_iter()
        .find(|p| p.symbol == ticket.symbol && p.direction == pos_dir)
    {
        if is_closing {
            let closed_qty = pos.quantity.min(ticket.quantity);
            let remaining_flip = (ticket.quantity - closed_qty).max(0.0);
            let sign = if pos_dir == "long" { 1.0 } else { -1.0 };
            pos.realized_pnl += (exec_price - pos.avg_entry_price) * closed_qty * sign - commission;
            pos.quantity = (pos.quantity - closed_qty).max(0.0);
            pos.updated_at = now;
            if pos.quantity <= 1e-9 {
                quant_port
                    .delete_position(&ticket.strategy_id, &ticket.symbol, pos_dir)
                    .await?;
            } else {
                quant_port.upsert_position(&pos).await?;
            }
            if remaining_flip > 1e-9 {
                let flip_dir = if pos_dir == "long" { "short" } else { "long" };
                let flipped = QuantPositionRecord {
                    id: new_id("pos"),
                    strategy_id: ticket.strategy_id.clone(),
                    symbol: ticket.symbol.clone(),
                    market: strat_market,
                    direction: flip_dir.into(),
                    quantity: remaining_flip,
                    avg_entry_price: exec_price,
                    realized_pnl: 0.0,
                    updated_at: now,
                };
                quant_port.upsert_position(&flipped).await?;
            }
        } else {
            let new_qty = pos.quantity + ticket.quantity;
            pos.avg_entry_price =
                (pos.quantity * pos.avg_entry_price + ticket.quantity * exec_price) / new_qty;
            pos.quantity = new_qty;
            pos.updated_at = now;
            quant_port.upsert_position(&pos).await?;
        }
    } else if !is_closing {
        let pos = QuantPositionRecord {
            id: new_id("pos"),
            strategy_id: ticket.strategy_id.clone(),
            symbol: ticket.symbol.clone(),
            market: strat_market,
            direction: pos_dir.into(),
            quantity: ticket.quantity,
            avg_entry_price: exec_price,
            realized_pnl: -commission,
            updated_at: now,
        };
        quant_port.upsert_position(&pos).await?;
    } else if ticket.quantity > 1e-9 {
        let open_dir = if pos_dir == "long" { "short" } else { "long" };
        let pos = QuantPositionRecord {
            id: new_id("pos"),
            strategy_id: ticket.strategy_id.clone(),
            symbol: ticket.symbol.clone(),
            market: strat_market,
            direction: open_dir.into(),
            quantity: ticket.quantity,
            avg_entry_price: exec_price,
            realized_pnl: -commission,
            updated_at: now,
        };
        quant_port.upsert_position(&pos).await?;
    }

    ticket.fill()?;
    quant_port.update_order_ticket(ticket).await?;
    Ok(fill)
}

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
            let direction = match ord.direction.trim().to_lowercase().as_str() {
                "buy" => OrderDirection::Buy,
                "sell" => OrderDirection::Sell,
                "sell_short" => OrderDirection::SellShort,
                "buy_to_cover" => OrderDirection::BuyToCover,
                other => {
                    return Err(DomainError::Validation(format!(
                        "Invalid order direction '{other}': expected buy, sell, sell_short, or buy_to_cover"
                    )));
                }
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

        let outbox_item = ticket.to_outbox_action_item(&req.task_id);
        let idempotency_key = format!(
            "quant:order:{}:{}:{}",
            req.task_id,
            ticket.id,
            &digest(outbox_item.payload_digest.as_bytes())[..16]
        );

        let mut settled_fill: Option<TradeFillRecord> = None;
        let mut receipt: Option<ExecutionReceipt> = None;
        let outbox_status = if req.execute_paper {
            let fill = settle_paper_fill(
                &mut ticket,
                &req.task_id,
                req.execution_price,
                None,
                quant_port,
            )
            .await?;
            let fill_val = serde_json::to_value(&fill).unwrap_or_default();
            receipt = Some(ExecutionReceipt {
                receipt_id: new_id("rcpt"),
                permit_id: req.permit_id.clone(),
                action_id: outbox_item.action_type.clone(),
                status: ReceiptStatus::Success,
                output_digest: format!("sha256:{}", digest(fill_val.to_string().as_bytes())),
                output_data: Some(fill_val),
                error_message: None,
                duration_ms: Some(1),
                executed_at: chrono::Utc::now(),
                assurance: Assurance::CustosMediated,
            });
            settled_fill = Some(fill);
            OutboxStatus::Receipted
        } else {
            OutboxStatus::Approved
        };

        if let Some(outbox) = outbox_port {
            let outbox_entry = OutboxEntry {
                id: outbox_item.id.clone(),
                task_id: req.task_id.clone(),
                action_id: outbox_item.action_type.clone(),
                permit_id: req.permit_id.clone(),
                argument_digest: outbox_item.payload_digest.clone(),
                idempotency_key: Some(idempotency_key.clone()),
                status: outbox_status,
                created_at: chrono::Utc::now(),
                receipt: receipt.clone(),
                recipient: Some(outbox_item.recipient.clone()),
                payload_json: outbox_item.payload_json.clone(),
                error_message: None,
            };
            // Never swallow outbox persistence failures on financial orders!
            outbox.enqueue(outbox_entry).await?;
        }

        let status_label = if settled_fill.is_some() {
            "filled"
        } else {
            "approved"
        };
        Ok(serde_json::json!({
            "approved_ticket": ticket,
            "status": status_label,
            "permit_id": req.permit_id,
            "idempotency_key": idempotency_key,
            "outbox_action_id": outbox_item.id,
            "fill": settled_fill,
        }))
    }

    /// Creates a real risk-reducing close-position order with Exit Immunity verified.
    pub async fn close_position(
        req: ClosePositionRequest,
        quant_port: &dyn QuantPort,
        outbox_port: Option<&dyn OutboxPort>,
    ) -> Result<serde_json::Value, DomainError> {
        let close_dir = if req.direction.eq_ignore_ascii_case("short") {
            OrderDirection::BuyToCover
        } else {
            OrderDirection::Sell
        };

        let ctx = DecisionContext::new(&req.symbol, close_dir, req.quantity, req.mark_price)?
            .with_risk_state(0.0, 0, 0.0, true);
        let gate = PreTradeDecisionGate::default();
        let outcome = gate.evaluate(&ctx);
        if !outcome.is_allowed() {
            return Err(DomainError::Validation(format!(
                "Close position rejected by pre-trade gate: {:?}",
                outcome
            )));
        }

        let mut ticket =
            OrderTicket::new(&req.strategy_id, &req.symbol, close_dir, req.quantity, 0.0)?
                .with_limit_price(req.mark_price);
        quant_port.save_order_ticket(&ticket, &req.task_id).await?;

        let mut fill = None;
        if let Some(permit_id) = req.permit_id.filter(|p| !p.trim().is_empty()) {
            ticket.approve_with_permit(permit_id.clone())?;
            quant_port.update_order_ticket(&ticket).await?;
            let outbox_item = ticket.to_outbox_action_item(&req.task_id);
            let idem = format!("quant:close:{}:{}", req.task_id, ticket.id);
            let status = if req.execute_paper {
                fill = Some(
                    settle_paper_fill(
                        &mut ticket,
                        &req.task_id,
                        Some(req.mark_price),
                        Some(CloseReason::ManualClose),
                        quant_port,
                    )
                    .await?,
                );
                OutboxStatus::Receipted
            } else {
                OutboxStatus::Approved
            };
            if let Some(outbox) = outbox_port {
                outbox
                    .enqueue(OutboxEntry {
                        id: outbox_item.id,
                        task_id: req.task_id,
                        action_id: "quant_position_close".into(),
                        permit_id,
                        argument_digest: outbox_item.payload_digest,
                        idempotency_key: Some(idem),
                        status,
                        created_at: chrono::Utc::now(),
                        receipt: None,
                        recipient: Some(outbox_item.recipient),
                        payload_json: outbox_item.payload_json,
                        error_message: None,
                    })
                    .await?;
            }
        }

        Ok(serde_json::json!({
            "gate_outcome": outcome,
            "ticket": ticket,
            "fill": fill,
        }))
    }

    pub async fn kill_switch(
        req: KillSwitchRequest,
        quant_port: &dyn QuantPort,
        outbox_port: Option<&dyn OutboxPort>,
    ) -> Result<serde_json::Value, DomainError> {
        let all_orders = quant_port.list_order_tickets(None, None).await?;

        let mut cancelled_count = 0usize;
        for mut ticket in all_orders {
            if matches!(
                ticket.status,
                OrderTicketStatus::Draft
                    | OrderTicketStatus::AwaitingPermit
                    | OrderTicketStatus::Approved
                    | OrderTicketStatus::Routed
            ) {
                ticket.cancel()?;
                quant_port.update_order_ticket(&ticket).await?;
                cancelled_count += 1;
            }
        }

        if let Some(outbox) = outbox_port {
            let now = chrono::Utc::now();
            let id = new_id("kill_switch");
            let entry = OutboxEntry {
                id: id.clone(),
                task_id: "emergency_stop".into(),
                action_id: "quant_emergency_stop".into(),
                permit_id: "emergency_operator_override".into(),
                argument_digest: digest(req.reason.as_bytes()),
                idempotency_key: Some(format!("quant:kill_switch:{id}")),
                // Honest status: locally cancelled pending tickets; broker cancellation requires external receipt
                status: OutboxStatus::Approved,
                created_at: now,
                receipt: None,
                recipient: Some("broker_execution_gateway".into()),
                payload_json: Some(
                    serde_json::json!({
                        "reason": req.reason,
                        "cancelled_orders": cancelled_count,
                        "scope": "local_pending_and_routed_tickets"
                    })
                    .to_string(),
                ),
                error_message: None,
            };
            outbox.enqueue(entry).await?;
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
        _quant_port: Option<&dyn QuantPort>,
    ) -> Result<OrderBookSnapshot, DomainError> {
        let _ = req.depth;
        Err(DomainError::Validation(format!(
            "Live L2 orderbook feed is not connected for '{}'; synthetic orderbook depth is disabled (Zero-Mock INV-11)",
            req.symbol
        )))
    }
}
