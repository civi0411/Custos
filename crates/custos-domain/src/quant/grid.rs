//! Native Grid Engine Domain Subsystem
//!
//! Models arithmetic and geometric grid level generation, cell pairing,
//! resting limit order placement, and realized volatility-harvesting PnL in Rust.

use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use super::order::OrderDirection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridMode {
    Arithmetic,
    Geometric,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridCell {
    pub index: usize,
    pub lower_price: f64,
    pub upper_price: f64,
    pub target_qty: f64,
    pub is_filled: bool,
    pub fill_count: usize,
    pub realized_pnl: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridConfig {
    pub symbol: String,
    pub lower_price: f64,
    pub upper_price: f64,
    pub grid_count: usize,
    pub mode: GridMode,
    pub total_capital: f64,
    pub stop_loss_price: Option<f64>,
    pub take_profit_price: Option<f64>,
}

impl GridConfig {
    pub fn new(
        symbol: impl Into<String>,
        lower_price: f64,
        upper_price: f64,
        grid_count: usize,
        mode: GridMode,
        total_capital: f64,
    ) -> Result<Self, DomainError> {
        let symbol = symbol.into();
        if symbol.trim().is_empty() {
            return Err(DomainError::Validation("Grid symbol cannot be empty".into()));
        }
        if lower_price <= 0.0 || upper_price <= lower_price {
            return Err(DomainError::Validation(format!(
                "Invalid price bounds: lower ({lower_price}) must be positive and less than upper ({upper_price})"
            )));
        }
        if grid_count < 2 || grid_count > 200 {
            return Err(DomainError::Validation(format!(
                "Grid count ({grid_count}) must be between 2 and 200"
            )));
        }
        if total_capital <= 0.0 {
            return Err(DomainError::Validation(
                "Total capital must be positive".into(),
            ));
        }

        Ok(Self {
            symbol,
            lower_price,
            upper_price,
            grid_count,
            mode,
            total_capital,
            stop_loss_price: None,
            take_profit_price: None,
        })
    }

    pub fn with_protection(mut self, stop_loss: Option<f64>, take_profit: Option<f64>) -> Self {
        self.stop_loss_price = stop_loss;
        self.take_profit_price = take_profit;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload", rename_all = "snake_case")]
pub enum GridOrderAction {
    PlaceLimit {
        client_order_id: String,
        cell_index: usize,
        side: OrderDirection,
        price: f64,
        quantity: f64,
    },
    CancelOrder {
        client_order_id: String,
        reason: String,
    },
    TriggerEmergencyStop {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridStatusSummary {
    pub symbol: String,
    pub active_cells: usize,
    pub filled_cells: usize,
    pub total_grid_cycles: usize,
    pub total_realized_pnl: f64,
    pub current_price: f64,
    pub in_boundary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeGridEngine {
    pub config: GridConfig,
    pub levels: Vec<f64>,
    pub cells: Vec<GridCell>,
    pub is_stopped: bool,
    pub stop_reason: Option<String>,
    pub total_cycles: usize,
    pub last_observed_price: f64,
}

impl NativeGridEngine {
    pub fn new(config: GridConfig) -> Result<Self, DomainError> {
        let levels = Self::generate_levels(
            config.lower_price,
            config.upper_price,
            config.grid_count,
            config.mode,
        )?;
        let cells = Self::generate_cells(&levels, config.total_capital);

        Ok(Self {
            config,
            levels,
            cells,
            is_stopped: false,
            stop_reason: None,
            total_cycles: 0,
            last_observed_price: 0.0,
        })
    }

    pub fn generate_levels(
        lower: f64,
        upper: f64,
        count: usize,
        mode: GridMode,
    ) -> Result<Vec<f64>, DomainError> {
        if count < 2 {
            return Err(DomainError::Validation("Grid count must be at least 2".into()));
        }
        if lower <= 0.0 || upper <= lower {
            return Err(DomainError::Validation(
                "Lower price must be positive and lower than upper price".into(),
            ));
        }

        let mut levels = Vec::with_capacity(count);
        match mode {
            GridMode::Arithmetic => {
                let step = (upper - lower) / (count - 1) as f64;
                for i in 0..count {
                    levels.push(lower + step * i as f64);
                }
            }
            GridMode::Geometric => {
                let ratio = (upper / lower).powf(1.0 / (count - 1) as f64);
                for i in 0..count {
                    levels.push(lower * ratio.powi(i as i32));
                }
            }
        }
        levels[0] = lower;
        levels[count - 1] = upper;
        Ok(levels)
    }

    pub fn generate_cells(levels: &[f64], total_capital: f64) -> Vec<GridCell> {
        if levels.len() < 2 {
            return Vec::new();
        }
        let cell_count = levels.len() - 1;
        let capital_per_cell = total_capital / cell_count as f64;

        let mut cells = Vec::with_capacity(cell_count);
        for i in 0..cell_count {
            let lower_price = levels[i];
            let upper_price = levels[i + 1];
            let mid_price = (lower_price + upper_price) * 0.5;
            let target_qty = if mid_price > 1e-8 {
                capital_per_cell / mid_price
            } else {
                0.0
            };

            cells.push(GridCell {
                index: i,
                lower_price,
                upper_price,
                target_qty,
                is_filled: false,
                fill_count: 0,
                realized_pnl: 0.0,
            });
        }
        cells
    }

    pub fn evaluate_price(&mut self, current_price: f64) -> Vec<GridOrderAction> {
        if self.is_stopped || current_price <= 0.0 {
            return Vec::new();
        }
        let prev_price = self.last_observed_price;
        self.last_observed_price = current_price;

        if prev_price <= 0.0 {
            return Vec::new();
        }

        // 1. Protection limits check
        if let Some(sl) = self.config.stop_loss_price {
            if current_price <= sl {
                self.is_stopped = true;
                let reason = format!("Stop loss breached at {current_price} <= {sl}");
                self.stop_reason = Some(reason.clone());
                return vec![GridOrderAction::TriggerEmergencyStop { reason }];
            }
        }
        if let Some(tp) = self.config.take_profit_price {
            if current_price >= tp {
                self.is_stopped = true;
                let reason = format!("Take profit reached at {current_price} >= {tp}");
                self.stop_reason = Some(reason.clone());
                return vec![GridOrderAction::TriggerEmergencyStop { reason }];
            }
        }

        let mut actions = Vec::new();

        // 2. Resting limit order generation based on price crossing cell boundaries
        for cell in &mut self.cells {
            let crossed_down = prev_price > cell.lower_price && current_price <= cell.lower_price;
            let crossed_up = prev_price < cell.upper_price && current_price >= cell.upper_price;

            if crossed_down && !cell.is_filled {
                cell.is_filled = true;
                cell.fill_count += 1;
                actions.push(GridOrderAction::PlaceLimit {
                    client_order_id: format!("grid-{}-long-entry-{}", cell.index, cell.fill_count),
                    cell_index: cell.index,
                    side: OrderDirection::Buy,
                    price: cell.lower_price,
                    quantity: cell.target_qty,
                });
            } else if crossed_up && cell.is_filled {
                cell.is_filled = false;
                cell.fill_count += 1;
                self.total_cycles += 1;
                let profit = (cell.upper_price - cell.lower_price) * cell.target_qty;
                cell.realized_pnl += profit;

                actions.push(GridOrderAction::PlaceLimit {
                    client_order_id: format!("grid-{}-short-exit-{}", cell.index, cell.fill_count),
                    cell_index: cell.index,
                    side: OrderDirection::Sell,
                    price: cell.upper_price,
                    quantity: cell.target_qty,
                });
            }
        }

        actions
    }

    pub fn total_realized_pnl(&self) -> f64 {
        self.cells.iter().map(|c| c.realized_pnl).sum()
    }

    pub fn status(&self) -> GridStatusSummary {
        let active_count = self.cells.iter().filter(|c| !c.is_filled).count();
        let filled_count = self.cells.iter().filter(|c| c.is_filled).count();
        let in_boundary = self.last_observed_price >= self.config.lower_price
            && self.last_observed_price <= self.config.upper_price;

        GridStatusSummary {
            symbol: self.config.symbol.clone(),
            active_cells: active_count,
            filled_cells: filled_count,
            total_grid_cycles: self.total_cycles,
            total_realized_pnl: self.total_realized_pnl(),
            current_price: self.last_observed_price,
            in_boundary,
        }
    }
}
