//! Execution Order Tickets & Trade Governance
//!
//! Models order tickets, execution constraints, algorithmic routing choices (TWAP, VWAP),
//! human approval gates, and seamless cross-workbench handoff into the governed Outbox.

use crate::error::DomainError;
use crate::ids::{digest, new_id};
use crate::session::assistant::OutboxActionItem;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderDirection {
    Buy,
    Sell,
    SellShort,
    BuyToCover,
}

impl std::fmt::Display for OrderDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderDirection::Buy => write!(f, "BUY"),
            OrderDirection::Sell => write!(f, "SELL"),
            OrderDirection::SellShort => write!(f, "SELL_SHORT"),
            OrderDirection::BuyToCover => write!(f, "BUY_TO_COVER"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Market,
    Limit,
    Stop,
    Twap,
    Vwap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderTicketStatus {
    Draft,
    AwaitingPermit,
    Approved,
    Rejected,
    Routed,
    Filled,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderTicket {
    pub id: String,
    pub strategy_id: String,
    pub symbol: String,
    pub direction: OrderDirection,
    pub order_type: OrderType,
    pub quantity: f64,
    pub limit_price: Option<f64>,
    pub target_weight: f64,
    pub estimated_slippage_bps: f64,
    pub approved_by_human: bool,
    pub status: OrderTicketStatus,
    pub permit_id: Option<String>,
    pub created_at: i64,
}

impl OrderTicket {
    pub fn new(
        strategy_id: impl Into<String>,
        symbol: impl Into<String>,
        direction: OrderDirection,
        quantity: f64,
        target_weight: f64,
    ) -> Result<Self, DomainError> {
        let strategy_id = strategy_id.into();
        let symbol = symbol.into();

        if strategy_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "OrderTicket strategy_id cannot be empty".into(),
            ));
        }
        if symbol.trim().is_empty() {
            return Err(DomainError::Validation(
                "OrderTicket symbol cannot be empty".into(),
            ));
        }
        if quantity <= 0.0 {
            return Err(DomainError::Validation(
                "OrderTicket quantity must be positive".into(),
            ));
        }

        let now = chrono::Utc::now().timestamp();
        Ok(Self {
            id: new_id("order"),
            strategy_id,
            symbol,
            direction,
            order_type: OrderType::Market,
            quantity,
            limit_price: None,
            target_weight,
            estimated_slippage_bps: 5.0,
            approved_by_human: false,
            status: OrderTicketStatus::Draft,
            permit_id: None,
            created_at: now,
        })
    }

    pub fn with_limit_price(mut self, limit_price: f64) -> Self {
        self.order_type = OrderType::Limit;
        self.limit_price = Some(limit_price);
        self
    }

    pub fn with_algorithmic_execution(mut self, order_type: OrderType, slippage_bps: f64) -> Self {
        self.order_type = order_type;
        self.estimated_slippage_bps = slippage_bps;
        self
    }

    pub fn transition_to_awaiting_permit(&mut self) -> Result<(), DomainError> {
        if self.status != OrderTicketStatus::Draft {
            return Err(DomainError::Validation(format!(
                "Cannot transition order {} to AwaitingPermit from status {:?}",
                self.id, self.status
            )));
        }
        self.status = OrderTicketStatus::AwaitingPermit;
        Ok(())
    }

    pub fn approve_with_permit(&mut self, permit_id: String) -> Result<(), DomainError> {
        if self.status != OrderTicketStatus::Draft
            && self.status != OrderTicketStatus::AwaitingPermit
        {
            return Err(DomainError::Validation(format!(
                "Cannot approve order {} from status {:?}",
                self.id, self.status
            )));
        }
        if permit_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Execution permit_id cannot be empty for approved order".into(),
            ));
        }
        self.approved_by_human = true;
        self.permit_id = Some(permit_id);
        self.status = OrderTicketStatus::Approved;
        Ok(())
    }

    pub fn reject(&mut self) -> Result<(), DomainError> {
        if self.status == OrderTicketStatus::Filled || self.status == OrderTicketStatus::Cancelled {
            return Err(DomainError::Validation(format!(
                "Cannot reject terminal order {} in status {:?}",
                self.id, self.status
            )));
        }
        self.approved_by_human = false;
        self.status = OrderTicketStatus::Rejected;
        Ok(())
    }

    pub fn route_to_venue(&mut self) -> Result<(), DomainError> {
        if self.status != OrderTicketStatus::Approved {
            return Err(DomainError::Validation(format!(
                "Cannot route unapproved order {} (current status: {:?})",
                self.id, self.status
            )));
        }
        self.status = OrderTicketStatus::Routed;
        Ok(())
    }

    pub fn fill(&mut self) -> Result<(), DomainError> {
        if self.status != OrderTicketStatus::Routed && self.status != OrderTicketStatus::Approved {
            return Err(DomainError::Validation(format!(
                "Cannot fill order {} from status {:?}",
                self.id, self.status
            )));
        }
        self.status = OrderTicketStatus::Filled;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), DomainError> {
        if self.status == OrderTicketStatus::Filled {
            return Err(DomainError::Validation(format!(
                "Cannot cancel already filled order {}",
                self.id
            )));
        }
        self.status = OrderTicketStatus::Cancelled;
        Ok(())
    }

    pub fn approve_by_human(&mut self, permit_id: Option<String>) {
        let p_id = permit_id.unwrap_or_else(|| "permit_auto".into());
        let _ = self.approve_with_permit(p_id);
    }

    pub fn reject_by_human(&mut self) {
        let _ = self.reject();
    }

    /// Converts this OrderTicket into an OutboxActionItem for governed Copilot outbox delivery.
    pub fn to_outbox_action_item(&self, task_id: &str) -> OutboxActionItem {
        let payload_digest = digest(
            format!(
                "{}:{}:{}:{}:{}",
                self.id, self.strategy_id, self.symbol, self.direction, self.quantity
            )
            .as_bytes(),
        );

        let payload_json = serde_json::to_string(self).ok();

        OutboxActionItem {
            id: new_id("outbox"),
            task_id: task_id.to_string(),
            title: format!(
                "Execute Order: {} {:.2} {} (Target: {:.2}%)",
                self.direction,
                self.quantity,
                self.symbol,
                self.target_weight * 100.0
            ),
            action_type: "quant_order_execution".to_string(),
            recipient: format!("BrokerVenue({})", self.strategy_id),
            payload_summary: format!(
                "Ticket {}: {} {} units of {} (Type: {:?}, Slippage: {:.1} bps)",
                self.id,
                self.direction,
                self.quantity,
                self.symbol,
                self.order_type,
                self.estimated_slippage_bps
            ),
            payload_digest,
            status: if self.approved_by_human {
                "approved".to_string()
            } else {
                "pending_approval".to_string()
            },
            permit_id: self.permit_id.clone(),
            receipt_id: None,
            payload_json,
            error_message: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
