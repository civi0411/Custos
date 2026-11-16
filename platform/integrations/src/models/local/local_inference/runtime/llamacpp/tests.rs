use llama_cpp_2::model::ChatTemplateResult;

use super::hardware::{
    check_cpu_supports_local_inference, unsupported_cpu_features_error_message,
};
use super::templates::{
    is_legacy_builtin_template_name, should_use_native_tool_calling,
    template_result_supports_native_tool_calling,
};
use crate::local_inference::model::ToolCallingMode;

fn template_result(parser: Option<&str>, parse_tool_calls: bool) -> ChatTemplateResult {
    ChatTemplateResult {
        prompt: String::new(),
        grammar: None,
        grammar_lazy: false,
        grammar_triggers: Vec::new(),
        preserved_tokens: Vec::new(),
        additional_stops: Vec::new(),
        chat_format: 0,
        parser: parser.map(str::to_string),
        generation_prompt: String::new(),
        parse_tool_calls,
    }
}

#[test]
fn native_tool_calling_requires_generated_parser() {
    assert!(template_result_supports_native_tool_calling(
        &template_result(Some("parser"), true)
    ));
    assert!(!template_result_supports_native_tool_calling(
        &template_result(None, true)
    ));
    assert!(!template_result_supports_native_tool_calling(
        &template_result(Some("parser"), false)
    ));
    assert!(!template_result_supports_native_tool_calling(
        &template_result(Some("   "), true)
    ));
}

#[test]
fn tool_calling_mode_controls_path_selection() {
    assert!(should_use_native_tool_calling(
        ToolCallingMode::Auto,
        true,
        true
    ));
    assert!(!should_use_native_tool_calling(
        ToolCallingMode::Auto,
        true,
        false
    ));
    assert!(should_use_native_tool_calling(
        ToolCallingMode::ForceNative,
        true,
        false
    ));
    assert!(!should_use_native_tool_calling(
        ToolCallingMode::ForceEmulated,
        true,
        true
    ));
    assert!(!should_use_native_tool_calling(
        ToolCallingMode::ForceNative,
        false,
        true
    ));
}

#[test]
fn rejects_legacy_builtin_names_as_inline_templates() {
    assert!(is_legacy_builtin_template_name("gemma"));
    assert!(is_legacy_builtin_template_name("llama3"));
    assert!(!is_legacy_builtin_template_name("chatml"));
    assert!(!is_legacy_builtin_template_name(
        "{% for message in messages %}{{ message.content }}{% endfor %}"
    ));
}

#[test]
fn local_inference_cpu_support_check_accepts_current_host() {
    let result = check_cpu_supports_local_inference();

    #[cfg(target_arch = "x86_64")]
    {
        let supports_required_features = std::arch::is_x86_feature_detected!("fma")
            && std::arch::is_x86_feature_detected!("avx2")
            && std::arch::is_x86_feature_detected!("f16c")
            && std::arch::is_x86_feature_detected!("bmi2")
            && std::arch::is_x86_feature_detected!("sse4.2");

        if supports_required_features {
            assert!(
                result.is_ok(),
                "expected supported x86_64 host to pass: {result:?}"
            );
        } else {
            assert!(result.is_err(), "expected unsupported x86_64 host to fail");
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    assert!(result.is_ok());
}

#[test]
fn local_inference_cpu_support_error_names_missing_instruction_sets() {
    let message = unsupported_cpu_features_error_message(&["FMA", "AVX2"]);

    assert!(message.contains("FMA"));
    assert!(message.contains("AVX2"));
}
