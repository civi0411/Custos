//! Custos Quantitative Trading Workbench (`custos-workbench-quant`)
//!
//! Autonomous alpha discovery, backtesting simulations, risk auditing,
//! and execution planning for algo finance agents.

pub mod contracts;
pub mod domain;
pub mod services;
pub mod execution;
pub mod jobs;
pub mod pack;
pub mod repository;
pub mod roles;
pub mod skills;
pub mod supervision;
pub mod verifiers;

pub use contracts::QuantPort;
pub use domain::*;
pub use services::{BacktestService, MarketService, OrderExecutionService};
pub use execution::*;
pub use jobs::*;
pub use pack::QuantPack;
pub use repository::QuantRepository;
pub use roles::*;
pub use skills::*;
pub use verifiers::*;

use custos_core::contracts::pack::{DomainPack, WorkbenchModule};
use custos_core::contracts::skill::SkillSink;
use std::sync::Arc;

/// Sovereign Workbench Module descriptor for Custos Quantitative Trading
#[derive(Default, Clone)]
pub struct QuantModule;

impl QuantModule {
    pub fn new() -> Self {
        Self
    }
}

impl WorkbenchModule for QuantModule {
    fn id(&self) -> &'static str {
        "quant"
    }

    fn pack(&self) -> Arc<dyn DomainPack> {
        Arc::new(QuantPack::new())
    }

    fn register_skills(&self, registry: &mut dyn SkillSink) {
        skills::register_quant_skills(registry);
    }
}

