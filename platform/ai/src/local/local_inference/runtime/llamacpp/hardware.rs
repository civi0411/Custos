use anyhow::Result;
use llama_cpp_2::{list_llama_ggml_backend_devices, LlamaBackendDeviceType};

#[cfg(any(target_arch = "x86_64", test))]
pub(crate) fn unsupported_cpu_features_error_message(missing_features: &[&str]) -> String {
    format!(
        "Local inference with the bundled llama.cpp backend requires CPU support for {}. \
         This CPU is missing {}. Use a CPU with those instruction sets or switch to a non-local provider.",
        missing_features.join(" and "),
        missing_features.join(", ")
    )
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn check_cpu_supports_local_inference() -> Result<()> {
    let missing_features = [
        (!std::arch::is_x86_feature_detected!("fma")).then_some("FMA"),
        (!std::arch::is_x86_feature_detected!("avx2")).then_some("AVX2"),
        (!std::arch::is_x86_feature_detected!("f16c")).then_some("F16C"),
        (!std::arch::is_x86_feature_detected!("bmi2")).then_some("BMI2"),
        (!std::arch::is_x86_feature_detected!("sse4.2")).then_some("SSE4.2"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if missing_features.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(unsupported_cpu_features_error_message(
            &missing_features
        )))
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub(crate) fn check_cpu_supports_local_inference() -> Result<()> {
    Ok(())
}

pub(crate) fn is_accelerator_device(device_type: LlamaBackendDeviceType) -> bool {
    matches!(
        device_type,
        LlamaBackendDeviceType::Gpu
            | LlamaBackendDeviceType::IntegratedGpu
            | LlamaBackendDeviceType::Accelerator
    )
}

pub(crate) fn is_non_cpu_device(device_type: LlamaBackendDeviceType) -> bool {
    !matches!(device_type, LlamaBackendDeviceType::Cpu)
}

pub(crate) fn log_inference_backend_devices() {
    let devices = list_llama_ggml_backend_devices();
    let non_cpu_devices: Vec<_> = devices
        .iter()
        .filter(|device| is_non_cpu_device(device.device_type))
        .collect();

    if non_cpu_devices.is_empty() {
        tracing::info!(
            device_count = devices.len(),
            "No non-CPU llama.cpp backend devices detected for local inference"
        );
        return;
    }

    for device in non_cpu_devices {
        tracing::info!(
            index = device.index,
            backend = %device.backend,
            name = %device.name,
            description = %device.description,
            device_type = ?device.device_type,
            memory_total_bytes = device.memory_total as u64,
            memory_free_bytes = device.memory_free as u64,
            "Non-CPU llama.cpp backend device detected for local inference"
        );
    }
}
