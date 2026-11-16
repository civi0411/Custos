//! Workbench Contribution Descriptor for Quant
//!
//! Exposes UI pane declarations, tool definitions, verification rules,
//! and workbench integration endpoints for quantitative finance.

use std::sync::Arc;
use custos_contracts::{
    contributions::pack::{DomainPack, WorkbenchModule},
    conversation::session::WorkbenchLens,
};
use crate::{QuantModule, QuantPack, pack::QuantPackDescriptor};

pub struct QuantWorkbenchContribution;

impl QuantWorkbenchContribution {
    pub fn module() -> Arc<dyn WorkbenchModule> {
        Arc::new(QuantModule::new())
    }

    pub fn pack() -> Arc<dyn DomainPack> {
        Arc::new(QuantPack::new())
    }

    pub fn lens() -> WorkbenchLens {
        WorkbenchLens::Quant
    }

    pub fn descriptor() -> QuantPackDescriptor {
        crate::pack::get_descriptor()
    }
}
