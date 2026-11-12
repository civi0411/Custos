//! Fleet & Automation Persistence Repository
//!
//! SQLite-backed repository managing:
//! - Remote SSH fleet nodes and command execution receipts
//! - Headless automation jobs and execution logs

mod fleet_executions;
mod headless_jobs;
mod remote_hosts;

#[cfg(test)]
mod tests;

use crate::connection::DbConnection;

#[derive(Clone)]
pub struct FleetAutomationRepository {
    pub(crate) db: DbConnection,
}

impl FleetAutomationRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
}

