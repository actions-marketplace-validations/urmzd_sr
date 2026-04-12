// Re-export agentspec-provider types under the names sr uses internally.
pub use agentspec_provider::{
    AiEvent, AiProvider as AiBackend, AiRequest, AiResponse, AiUsage, Capability,
    LocalBackend as Backend, ProviderConfig, Sandbox, resolve_local_provider,
};

/// CLI-facing config — maps to ProviderConfig with sr's read-only sandbox.
pub struct BackendConfig {
    pub backend: Option<Backend>,
    pub model: Option<String>,
    pub budget: f64,
    pub debug: bool,
}

impl BackendConfig {
    /// Convert to agentspec-provider's ProviderConfig with sr's read-only sandbox.
    pub fn to_provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            backend: self.backend,
            model: self.model.clone(),
            budget: Some(self.budget),
            sandbox: Some(Sandbox {
                allowed: vec![Capability::GitReadOnly, Capability::ReadFile],
                denied: vec![
                    Capability::WriteFile,
                    Capability::ShellCommand {
                        pattern: ".*".into(),
                    },
                ],
            }),
            debug: self.debug,
        }
    }
}

pub async fn resolve_backend(config: &BackendConfig) -> anyhow::Result<Box<dyn AiBackend>> {
    resolve_local_provider(config.to_provider_config()).await
}
