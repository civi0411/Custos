//! Custos Session-Task Bridge
//!
//! Connects interactive sessions with durable autonomous tasks through:
//! - Promotion: Elevating an interactive conversation into a verifiable task
//! - Attachment: Linking a live session to an existing task for observation/steering
//! - Steering: Sending mid-flight guidance from a session into an active task

pub mod port;
pub mod service;

pub use port::{
    AttachMode, BridgeError, BridgePort, ForkReceipt, RecallReceipt, SteerReceipt, SteerResult,
    TaskObservation,
};
pub use service::BridgeService;
