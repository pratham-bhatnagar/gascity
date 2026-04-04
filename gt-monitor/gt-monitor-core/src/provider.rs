use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Capability, Query, QueryResult, Result};

/// The core abstraction for data sources. Every provider implements this trait
/// to expose one or more capabilities to the engine.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Human-readable provider name (e.g. "gitea", "dolt", "system").
    fn name(&self) -> &str;

    /// The set of capabilities this provider can serve.
    fn capabilities(&self) -> Vec<Capability>;

    /// Check provider health and connectivity.
    async fn health(&self) -> ProviderHealth;

    /// Execute a query. The caller guarantees `q.capability` is in `self.capabilities()`.
    async fn query(&self, q: &Query) -> Result<QueryResult>;

    /// Initialize the provider with configuration. Called once at startup.
    async fn init(&mut self, config: &ProviderConfig) -> Result<()>;
}

/// Provider health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub name: String,
    pub status: ProviderStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

/// Whether a provider is operational.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Healthy,
    Degraded,
    Unavailable,
}

/// Configuration passed to a provider during init.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(flatten)]
    pub settings: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_status_serializes() {
        let json = serde_json::to_string(&ProviderStatus::Healthy).unwrap();
        assert_eq!(json, r#""healthy""#);

        let json = serde_json::to_string(&ProviderStatus::Unavailable).unwrap();
        assert_eq!(json, r#""unavailable""#);
    }

    #[test]
    fn provider_health_serializes() {
        let h = ProviderHealth {
            name: "test".into(),
            status: ProviderStatus::Healthy,
            latency_ms: Some(5),
            message: None,
        };
        let json = serde_json::to_string(&h).unwrap();
        assert!(json.contains(r#""status":"healthy""#));
    }

    #[test]
    fn provider_config_from_map() {
        let json = r#"{"url":"http://localhost:3300","token":"abc"}"#;
        let cfg: ProviderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(
            cfg.settings.get("url").and_then(|v| v.as_str()),
            Some("http://localhost:3300")
        );
    }
}
