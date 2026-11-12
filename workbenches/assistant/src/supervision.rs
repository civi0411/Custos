//! Supervision policy tailored to the Assistant Workbench.

use custos_domain::supervision::{StepSpanKind, SupervisionProfile, ThresholdPolicy};

pub fn assistant_supervision_profile() -> SupervisionProfile {
    SupervisionProfile::for_workbench(
        "assistant",
        "Copilot/Assistant supervision: suspends stall alarms during human-in-the-loop approvals",
        ThresholdPolicy {
            long_running_ms: 90_000,
            repeated_tool_count: 4,
            repeated_error_count: 3,
            no_activity_ms: 60_000,
            possible_loop_window: 12,
            loop_repeat_count: 3,
        },
    )
    .with_exempt_kinds(vec![StepSpanKind::Session])
}

