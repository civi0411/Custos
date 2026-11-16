use super::context::{context_cap, effective_context_size};
use crate::local_inference::model::ModelSettings;

fn default_settings() -> ModelSettings {
    ModelSettings::default()
}

#[test]
fn test_effective_context_size_uses_full_limit() {
    assert_eq!(
        effective_context_size(100, &default_settings(), 4096, 4096, None),
        4096
    );
}

#[test]
fn test_effective_context_size_capped_by_limit() {
    assert_eq!(
        effective_context_size(100, &default_settings(), 1024, 8192, None),
        1024
    );
}

#[test]
fn test_effective_context_size_capped_by_memory() {
    assert_eq!(
        effective_context_size(100, &default_settings(), 4096, 4096, Some(800)),
        800
    );
}

#[test]
fn test_effective_context_size_memory_smaller_than_prompt() {
    assert_eq!(
        effective_context_size(600, &default_settings(), 4096, 4096, Some(700)),
        700
    );
}

#[test]
fn test_effective_context_size_zero_limit_uses_train() {
    assert_eq!(
        effective_context_size(100, &default_settings(), 0, 2048, None),
        2048
    );
}

#[test]
fn test_effective_context_size_prompt_exceeds_all_limits() {
    assert_eq!(
        effective_context_size(5000, &default_settings(), 4096, 4096, None),
        4096
    );
}

#[test]
fn test_context_cap_with_settings_override() {
    let mut settings = default_settings();
    settings.context_size = Some(2048);
    assert_eq!(context_cap(&settings, 4096, 8192, Some(1024)), 2048);
}

#[test]
fn test_context_cap_without_override() {
    assert_eq!(context_cap(&default_settings(), 4096, 8192, None), 4096);
}

#[test]
fn test_context_cap_memory_limited() {
    assert_eq!(
        context_cap(&default_settings(), 4096, 8192, Some(2048)),
        2048
    );
}
