//! Supervision policy tailored to the Research Workbench.

use custos_domain::supervision::{StepSpanKind, SupervisionProfile, ThresholdPolicy};

pub fn research_supervision_profile() -> SupervisionProfile {
    SupervisionProfile::for_workbench(
        "research",
        "Scientific research supervision: allows extended notebook execution and literature sweeps",
        ThresholdPolicy {
            long_running_ms: 300_000,
            repeated_tool_count: 6,
            repeated_error_count: 3,
            no_activity_ms: 90_000,
            possible_loop_window: 12,
            loop_repeat_count: 3,
        },
    )
    .with_exempt_kinds(vec![
        StepSpanKind::Session,
        StepSpanKind::Harness,
        StepSpanKind::Verification,
    ])
}

