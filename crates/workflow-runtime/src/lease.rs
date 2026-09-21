//! Task Lease Management

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TaskLease {
    pub task_id: String,
    pub holder_id: String,
    pub ttl: Duration,
}
