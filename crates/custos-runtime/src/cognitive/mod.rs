//! Custos Cognitive Runtime
//!
//! Coordinates System One (instant judgment) and System Two (deep deliberation)
//! under the Resolve-Delegate-Check (RDC) protocol.

pub mod approval_router;
pub mod arbiter;
pub mod config;
pub mod deliberation;
pub mod human_gate;
pub mod pipeline;
pub mod prompt_manager;
pub mod provider_selector;
pub mod rdc;
pub mod registry;
pub mod routing;
pub mod signal_extractor;

pub use approval_router::*;
pub use arbiter::*;
pub use config::*;
pub use deliberation::*;
pub use human_gate::*;
pub use pipeline::*;
pub use prompt_manager::*;
pub use provider_selector::*;
pub use rdc::*;
pub use registry::*;
pub use routing::*;
pub use signal_extractor::*;
