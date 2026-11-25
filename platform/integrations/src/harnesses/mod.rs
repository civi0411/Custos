pub mod adapter;
pub mod binary;
pub mod catalog;
#[cfg(any(test, feature = "test-support"))]
pub mod mock;
pub mod observer;
pub mod registry;
pub mod spec;
pub mod stream;

pub use adapter::{
    ClaudeCodeHarnessAdapter, CliAgentAdapter, CliHarness, CodexHarnessAdapter,
    GenericCliHarnessAdapter, GooseHarnessAdapter, OpenCodeHarnessAdapter,
};
pub use binary::{probe_binary_available, resolve_binary_path};
pub use catalog::{find_spec, SPECS};
#[cfg(any(test, feature = "test-support"))]
pub use mock::MockHarnessState;
pub use observer::{CancellableObserver, NoopObserver};
pub use registry::*;
pub use spec::{HarnessSpec, HeadlessArgs, StreamProtocol};
pub use stream::extract_observed_intents;
