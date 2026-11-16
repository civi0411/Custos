//! Workbench Contribution Descriptor for Research
//!
//! Exposes UI pane declarations, tool definitions, verification rules,
//! and workbench integration endpoints for academic and scientific research.

use std::sync::Arc;
use custos_contracts::{
    contributions::pack::{DomainPack, WorkbenchModule},
    conversation::session::WorkbenchLens,
};
use crate::{ResearchModule, ResearchPack, ResearchPackDescriptor};

pub struct ResearchWorkbenchContribution;

impl ResearchWorkbenchContribution {
    pub fn module() -> Arc<dyn WorkbenchModule> {
        Arc::new(ResearchModule::new())
    }

    pub fn pack() -> Arc<dyn DomainPack> {
        Arc::new(ResearchPack::new())
    }

    pub fn lens() -> WorkbenchLens {
        WorkbenchLens::Research
    }

    pub fn descriptor() -> ResearchPackDescriptor {
        crate::get_descriptor()
    }
}
