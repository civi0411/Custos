//! Workbench Contribution Descriptor for Coding
//!
//! Exposes UI pane declarations, tool definitions, verification rules,
//! and workbench integration endpoints for software engineering.

use std::sync::Arc;
use custos_contracts::{
    contributions::pack::{DomainPack, WorkbenchModule},
    conversation::session::WorkbenchLens,
};
use crate::{CodingModule, EngineeringPack, EngineeringPackDescriptor};

pub struct CodingWorkbenchContribution;

impl CodingWorkbenchContribution {
    pub fn module() -> Arc<dyn WorkbenchModule> {
        Arc::new(CodingModule::new())
    }

    pub fn pack() -> Arc<dyn DomainPack> {
        Arc::new(EngineeringPack::new())
    }

    pub fn lens() -> WorkbenchLens {
        WorkbenchLens::Coding
    }

    pub fn descriptor() -> EngineeringPackDescriptor {
        crate::get_descriptor()
    }
}
