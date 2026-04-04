use crate::Capability;

/// Errors returned by the gt-monitor engine and providers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no provider registered for capability {0:?}")]
    NoProvider(Capability),

    #[error("provider {provider} failed: {message}")]
    ProviderError { provider: String, message: String },

    #[error("invalid query: {0}")]
    InvalidQuery(String),

    #[error("provider {provider} not initialized")]
    NotInitialized { provider: String },

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
