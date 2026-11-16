//! Workbench Contribution Descriptor for Assistant
//!
//! Exposes UI pane declarations, tool definitions, verification rules,
//! and workbench integration endpoints.

use std::sync::Arc;
use custos_contracts::{
    contributions::pack::{DomainPack, WorkbenchModule},
    conversation::session::WorkbenchLens,
};
use crate::{AssistantModule, AssistantPack};

pub struct AssistantWorkbenchContribution;

impl AssistantWorkbenchContribution {
    pub fn module() -> Arc<dyn WorkbenchModule> {
        Arc::new(AssistantModule::new())
    }

    pub fn pack() -> Arc<dyn DomainPack> {
        Arc::new(AssistantPack::new())
    }

    pub fn lens() -> WorkbenchLens {
        WorkbenchLens::Assistant
    }

    pub fn descriptor() -> crate::AssistantPackDescriptor {
        crate::get_descriptor()
    }
}
