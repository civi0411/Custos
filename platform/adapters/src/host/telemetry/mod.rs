//! Host OS process/port telemetry and OTLP execution trace/log protocol normalization.

pub mod host_metrics;
pub mod otlp;

pub use host_metrics::*;
pub use otlp::{normalize_otlp_logs_json, normalize_otlp_traces_json};

