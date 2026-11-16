mod metadata;
mod providers;

#[cfg(test)]
mod tests;

pub use metadata::{
    get_setup_catalog_entries, ProviderSetupCapabilities, ProviderSetupCatalogEntry,
    ProviderSetupCategory, ProviderSetupField, ProviderSetupFieldOverride, ProviderSetupGroup,
    ProviderSetupMetadata, ProviderSetupMethod,
};

#[cfg(test)]
pub(crate) use metadata::setup_entry_from_metadata;

pub use providers::{
    get_provider_template, get_providers_by_format, ModelTemplate, ProviderCatalogEntry,
    ProviderFormat, ProviderTemplate, TemplateModelCapabilities,
};
