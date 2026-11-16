#[macro_use]
mod macros;

pub mod builder;
pub mod types;

use include_dir::{include_dir, Dir};

pub use builder::{
    config_from_json, deserialize_provider_config, from_json, load_custom_providers,
};
pub use types::{
    default_requires_auth, should_preserve_thinking_by_default, AuthConfig,
    DeclarativeProviderConfig, EnvKeyResolver, EnvVarConfig, KeyResolver, ProviderEngine,
};

pub static FIXED_PROVIDERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/data/providers");

pub(crate) mod declarative_providers {
    use super::*;

    expose_declarative_providers!(
        aimlapi,
        alibaba,
        atomic_chat,
        celeris,
        cerebras,
        deepseek,
        empiriolabs,
        eurouter,
        fireworks,
        friendli,
        futurmix,
        groq,
        iflytek,
        iflytek_astron,
        inception,
        llama_swap,
        lmstudio,
        lynkr,
        meta,
        minimax,
        mistral,
        moonshot,
        nearai,
        novita,
        nvidia,
        ollama_cloud,
        omlx,
        opencode_go,
        opencode_zen,
        opper,
        orcarouter,
        ovhcloud,
        perplexity,
        pleumrouter,
        routstr,
        sakana,
        saladcloud,
        saygm,
        scaleway,
        tanzu,
        tensorix,
        together,
        trustedrouter,
        venice,
        vercel_ai_gateway,
        zai,
        zai_coding_plan,
        zhipu,
    );
}

pub fn fixed_provider_configs() -> anyhow::Result<Vec<DeclarativeProviderConfig>> {
    declarative_providers::fixed_provider_configs()
}

pub fn fixed_provider_config_entries() -> Vec<(&'static str, &'static str)> {
    declarative_providers::fixed_provider_config_entries()
}

#[cfg(test)]
mod tests;
