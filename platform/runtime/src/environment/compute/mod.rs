//! Sovereign Compute & Hardware Workload Runtime (GPU-A / GPU-B / GPU-C / GPU-D / GPU-E / GPU-F)

pub mod admission;
pub mod evaluation;
pub mod placement;
pub mod relay;
pub mod remote;
pub mod training;

pub use admission::WorkloadAdmissionCoordinator;
pub use evaluation::ModelEvaluationRunner;
pub use placement::OptionalConsumerPlacementEngine;
pub use relay::CheckpointRelayCoordinator;
pub use remote::RemoteExecutionCoordinator;
pub use training::LocalTrainingRunner;
