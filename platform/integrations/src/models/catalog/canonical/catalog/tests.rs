use super::*;
use crate::contracts::types::base::{ConfigKey, ProviderMetadata};

fn setup_provider(
    name: &str,
    display_name: &str,
    category: ProviderSetupCategory,
    group: ProviderSetupGroup,
) -> ProviderMetadata {
    ProviderMetadata::new(name, display_name, "", "", vec![], "", vec![]).with_setup(
        ProviderSetupMetadata::new(category, ProviderSetupMethod::ConfigFields, group),
    )
}

#[test]
fn setup_catalog_has_stable_presentation_order() {
    let entries = get_setup_catalog_entries(vec![
        setup_provider(
            "gamma",
            "Gamma",
            ProviderSetupCategory::Model,
            ProviderSetupGroup::Additional,
        ),
        setup_provider(
            "beta",
            "Beta",
            ProviderSetupCategory::Model,
            ProviderSetupGroup::Default,
        ),
        setup_provider(
            "alpha",
            "Alpha",
            ProviderSetupCategory::Agent,
            ProviderSetupGroup::Default,
        ),
    ]);

    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.provider_id.as_str())
            .collect::<Vec<_>>(),
        ["custos-engine", "alpha", "beta", "gamma"]
    );
}

#[test]
fn single_api_key_uses_api_key_presentation() {
    let metadata = ProviderMetadata::new(
        "example",
        "Example",
        "",
        "",
        vec![],
        "",
        vec![ConfigKey::new("EXAMPLE_TOKEN", true, true, None, true)],
    )
    .with_setup(ProviderSetupMetadata::api_key(ProviderSetupGroup::Default));

    let entry = setup_entry_from_metadata(metadata).unwrap();
    assert_eq!(entry.fields[0].label, "API Key");
    assert_eq!(
        entry.fields[0].placeholder.as_deref(),
        Some("Paste your API key")
    );
}

#[test]
fn other_secret_fields_do_not_claim_to_be_api_keys() {
    let metadata = ProviderMetadata::new(
        "example",
        "Example",
        "",
        "",
        vec![],
        "",
        vec![ConfigKey::new("EXAMPLE_TOKEN", true, true, None, true)],
    )
    .with_setup(ProviderSetupMetadata::new(
        ProviderSetupCategory::Model,
        ProviderSetupMethod::ConfigFields,
        ProviderSetupGroup::Default,
    ));

    let entry = setup_entry_from_metadata(metadata).unwrap();
    assert_eq!(entry.fields[0].label, "Example Token");
    assert_eq!(entry.fields[0].placeholder, None);
}
