pub mod antigravity;
pub mod catalog;
pub mod claude;
pub mod codex;
pub mod fake;
pub mod local_model;
pub mod pricing;
pub mod probe;
#[allow(clippy::module_inception)]
pub mod providers;

pub use antigravity::*;
pub use catalog::*;
pub use claude::*;
pub use codex::*;
pub use fake::*;
pub use local_model::*;
pub use pricing::*;
pub use probe::*;
pub use providers::*;

#[cfg(test)]
mod placeholder_tests {
    use super::*;
    use custos_provider::{ModelProvider, ProviderRequest};

    #[tokio::test]
    async fn unconfigured_adapters_never_return_fabricated_model_output() {
        let request = ProviderRequest::simple("Explain the repository");
        let adapters: Vec<Box<dyn ModelProvider>> = vec![
            Box::new(CodexProvider::new()),
            Box::new(ClaudeProvider::new()),
            Box::new(AntigravityProvider::new()),
            Box::new(LocalModelProvider::new()),
        ];

        for adapter in adapters {
            assert!(
                adapter.generate(&request).await.is_err(),
                "{} returned fabricated output",
                adapter.provider_id()
            );
        }
    }
}
