// Re-export agentspec-provider types under the names sr uses internally.
pub use agentspec_provider::{
    AiEvent, AiProvider as AiBackend, AiRequest, AiResponse, AiUsage,
    Backend, ProviderConfig, resolve_provider,
};

/// CLI-facing config — maps to ProviderConfig.
pub struct BackendConfig {
    pub backend: Option<Backend>,
    pub model: Option<String>,
    pub debug: bool,
}

impl BackendConfig {
    /// Convert to agentspec-provider's ProviderConfig.
    pub fn to_provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            backend: self.backend,
            model: self.model.clone(),
            debug: self.debug,
        }
    }
}

pub async fn resolve_backend(config: &BackendConfig) -> anyhow::Result<Box<dyn AiBackend>> {
    resolve_provider(config.to_provider_config()).await
}
