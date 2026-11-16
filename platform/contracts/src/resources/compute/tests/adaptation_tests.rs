use crate::fleet::compute::adaptation::*;
use crate::fleet::compute::backend::*;
use crate::fleet::compute::checkpoint::*;

#[test]
fn invalid_hardware_dimensions_do_not_panic_or_fabricate_a_plan() {
    let gpu = AcceleratorSpec::tesla_t4();
    let checkpoint = TrainingCheckpointManifest::new("c", 0, 0.0, "s3://b/c", "");
    for (batch, accum) in [(0, 1), (1, 0), (u32::MAX, 2)] {
        assert!(
            HardwareAdaptationPlan::evaluate_gate(&gpu, &gpu, batch, accum, &checkpoint)
                .plan
                .is_none()
        );
    }
    let tiny = AcceleratorSpec::new("tiny", 1, false);
    assert!(HardwareAdaptationPlan::compute(&gpu, &tiny, 1, 1, &checkpoint).is_err());
}

#[test]
fn test_hardware_adaptation_a100_to_t4() {
    let source_a100 = AcceleratorSpec::a100_40gb();
    let target_t4 = AcceleratorSpec::tesla_t4();

    let checkpoint = TrainingCheckpointManifest::new(
        "ckpt_001",
        450,
        1.5,
        "hf://user/repo/checkpoint-450",
        "sha256_mock_hash",
    )
    .with_precision("bf16");

    let eval =
        HardwareAdaptationPlan::evaluate_gate(&source_a100, &target_t4, 8, 2, &checkpoint);

    assert!(matches!(
        eval.verdict,
        CompatibilityVerdict::RequiresAdaptation { .. }
    ));
    assert_eq!(eval.target_precision, "fp16");

    let plan = eval.plan.expect("plan should be available");
    assert!(plan.precision_downgraded);
    assert_eq!(plan.target_precision, "fp16");
    assert_eq!(plan.per_device_batch_size, 3);
    assert_eq!(plan.gradient_accumulation_steps, 6);
    assert_eq!(plan.resume_step, 450);
    assert_eq!(plan.resume_checkpoint_uri, "hf://user/repo/checkpoint-450");
}

#[test]
fn test_hardware_adaptation_t4_to_4090() {
    let source_t4 = AcceleratorSpec::tesla_t4();
    let target_4090 = AcceleratorSpec::rtx_4090();

    let checkpoint = TrainingCheckpointManifest::new(
        "ckpt_002",
        100,
        0.5,
        "s3://bucket/checkpoints/step-100",
        "sha256_hash",
    )
    .with_full_state("meta-llama/Llama-3-8B", "unsloth-2024.8", "train:100/1000");

    let eval =
        HardwareAdaptationPlan::evaluate_gate(&source_t4, &target_4090, 4, 4, &checkpoint);

    assert_eq!(eval.verdict, CompatibilityVerdict::Compatible);

    let plan = eval.plan.expect("plan must exist");
    assert!(!plan.precision_downgraded);
    assert_eq!(plan.per_device_batch_size, 4);
    assert_eq!(plan.gradient_accumulation_steps, 4);
    assert_eq!(plan.effective_batch_size, 16);
}

#[test]
fn test_compatibility_gate_insufficient_vram() {
    let source = AcceleratorSpec::rtx_4090();
    let tiny_gpu = AcceleratorSpec::new("Tiny Embedded GPU", 2048, false);

    let checkpoint =
        TrainingCheckpointManifest::new("ckpt_tiny", 50, 0.1, "s3://bucket/test", "hash");

    let eval = HardwareAdaptationPlan::evaluate_gate(&source, &tiny_gpu, 2, 2, &checkpoint);
    assert!(matches!(
        eval.verdict,
        CompatibilityVerdict::Incompatible { .. }
    ));
    assert!(eval.plan.is_none());
}

