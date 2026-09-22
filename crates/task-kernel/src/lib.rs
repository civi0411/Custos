//! Custos Task Kernel
//!
//! Event-driven CQRS task orchestration engine enforcing business invariants,
//! state machine transitions, and audit-ready domain events.

pub mod commands;
pub mod completion;
pub mod events;
pub mod invariants;
pub mod ports;
pub mod reducer;
pub mod service;
pub mod span_service;
pub mod state_machine;

pub use commands::*;
pub use completion::CompletionGate;
pub use events::*;
pub use invariants::TaskInvariants;
pub use ports::TaskStore;
pub use reducer::TaskReducer;
pub use service::TaskService;
pub use span_service::SpanService;
pub use state_machine::TaskStateMachine;
