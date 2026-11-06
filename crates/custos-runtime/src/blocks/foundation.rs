//! Foundation Block Runtime
//!
//! Provides core runtime infrastructure: high-resolution monotonic clocks,
//! asynchronous event bus, integration registries, and distributed task DAG engines.

pub use crate::clock::{ClockError, ClockTick, PrecisionEventClock};
pub use crate::compute::placement::OptionalConsumerPlacementEngine;
pub use crate::event_bus::{EventBusError, PipelineEvent, PipelineEventBus};
pub use crate::integration::{
    BindingDirection, IntegrationBinding, IntegrationKind, IntegrationRegistry, LifecycleState,
    MediationLevel,
};
pub use crate::packages::WorkerLease;
pub use crate::workflow::dag::{DagGraph, DagNode, ExecutionWave, NodeStatus};
pub use crate::workflow::graph_runtime::{GraphExecutionReport, GraphRuntime, StepExecutor};
pub use crate::workflow::machine::{
    EffectExecutor, OperationOutcome, WorkflowMachine, WorkflowOperation, WorkflowState,
    WorkflowStep,
};
pub use crate::workflow::scheduler::{
    JobRecurrence, JobStatus, ScheduledJob, ScheduledJobId, WorkflowScheduler,
};
pub use crate::workflow::TaskRuntime;
