//! Orchestration Intelligence (OI) Domain Models (RFC 003 / RFC 004 / RFC 005)
//!
//! Pure, deterministic, zero-I/O vocabulary for deliberation, topology selection,
//! candidate admissibility, work packets, and replanning.

pub mod candidate;
pub mod placement;
pub mod proposal;
pub mod record;
pub mod replan;
pub mod snapshot;
pub mod topology;
pub mod work_packet;

pub use candidate::{Candidate, RejectionReason};
pub use placement::NodePlacement;
pub use proposal::StrategyProposal;
pub use record::DecisionRecord;
pub use replan::{Assumption, ReplanBrief, ReplanRecord, ReplanTrigger};
pub use snapshot::DecisionSnapshot;
pub use topology::ExecutionTopology;
pub use work_packet::{WorkPacket, WorkerResult, WorkerStatus};
