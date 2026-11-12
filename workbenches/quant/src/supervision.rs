//! Supervision policy tailored to the Quant Workbench.

use custos_domain::supervision::{StepSpanKind, SupervisionProfile, ThresholdPolicy};

pub fn quant_supervision_profile() -> SupervisionProfile {
    SupervisionProfile::for_workbench(
        "quant",
        "Quantitative finance supervision: long backtest tolerance with strict repeated-error alerting",
        ThresholdPolicy {
            long_running_ms: 900_000,
            repeated_tool_count: 4,
            repeated_error_count: 2,
            no_activity_ms: 120_000,
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

