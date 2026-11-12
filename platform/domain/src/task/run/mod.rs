//! Task Run Entity & Execution Sessions
//!
//! A Run represents an active execution attempt or session for a Task.
//! Telemetry, step spans, deterministic watchdog rules, and error signatures
//! are first-class canonical models belonging directly to the Run domain.

pub mod claim;
pub mod command;
pub mod finding;
pub mod launch;
pub mod live;
pub mod step_span;
pub mod types;
pub mod usage;
pub mod watchdog;

#[cfg(test)]
mod tests;

pub use claim::*;
pub use command::*;
pub use finding::*;
pub use launch::*;
pub use live::*;
pub use step_span::*;
pub use types::*;
pub use usage::*;
pub use watchdog::*;
