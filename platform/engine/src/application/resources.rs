//! Ephemeral Execution Resources & Coordinated Environment Application Service

use crate::resources::terminal::TerminalCoordinator;
use crate::resources::workspace::WorkspaceCoordinator;
use std::sync::Arc;

/// Application service orchestrating workspace leases, terminals, compute, and notebooks.
pub struct ResourceApplicationService {
    workspace_coordinator: Arc<WorkspaceCoordinator>,
    terminal_coordinator: Arc<TerminalCoordinator>,
}

impl ResourceApplicationService {
    pub fn new(
        workspace_coordinator: Arc<WorkspaceCoordinator>,
        terminal_coordinator: Arc<TerminalCoordinator>,
    ) -> Self {
        Self {
            workspace_coordinator,
            terminal_coordinator,
        }
    }

    pub fn workspace(&self) -> &Arc<WorkspaceCoordinator> {
        &self.workspace_coordinator
    }

    pub fn terminal(&self) -> &Arc<TerminalCoordinator> {
        &self.terminal_coordinator
    }
}
