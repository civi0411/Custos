//! Supervision policy tailored to the Coding Workbench.

use custos_domain::supervision::{StepSpanKind, SupervisionProfile, ThresholdPolicy};

pub fn coding_supervision_profile() -> SupervisionProfile {
    SupervisionProfile::for_workbench(
        "coding",
        "Software engineering supervision: fast loop detection and compiler/test stall tolerance",
        ThresholdPolicy {
            long_running_ms: 120_000,
            repeated_tool_count: 5,
            repeated_error_count: 3,
            no_activity_ms: 45_000,
            possible_loop_window: 12,
            loop_repeat_count: 3,
        },
    )
    .with_exempt_kinds(vec![StepSpanKind::Session, StepSpanKind::Verification])
}

