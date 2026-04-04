use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::{Capability, Error, Provider, ProviderHealth, Query, QueryResult, Result};

/// Configuration for the Engine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Identifier for this Gas Town instance. Tagged on all results.
    pub town_id: String,
}

/// Aggregate health of all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub town_id: String,
    pub providers: Vec<ProviderHealth>,
}

/// The query router. Maintains a registry of providers and an index mapping
/// capabilities to providers for O(1) dispatch.
pub struct Engine {
    providers: Vec<Arc<dyn Provider>>,
    capability_index: HashMap<Capability, Vec<usize>>,
    config: EngineConfig,
}

impl Engine {
    /// Build an engine from a set of initialized providers.
    pub fn new(providers: Vec<Arc<dyn Provider>>, config: EngineConfig) -> Self {
        let mut capability_index: HashMap<Capability, Vec<usize>> = HashMap::new();
        for (i, p) in providers.iter().enumerate() {
            for cap in p.capabilities() {
                capability_index.entry(cap).or_default().push(i);
            }
        }
        Engine {
            providers,
            capability_index,
            config,
        }
    }

    /// Execute a single query, routing to the first provider that supports the capability.
    pub async fn query(&self, q: &Query) -> Result<QueryResult> {
        let provider_indices = self
            .capability_index
            .get(&q.capability)
            .ok_or(Error::NoProvider(q.capability))?;

        let idx = provider_indices[0];
        let provider = &self.providers[idx];

        let start = Instant::now();
        let mut result = provider.query(q).await?;
        result.latency_ms = start.elapsed().as_millis() as u64;
        result.provider = provider.name().to_string();
        Ok(result)
    }

    /// Execute multiple queries in parallel, fanning out via tokio.
    pub async fn query_many(&self, queries: Vec<Query>) -> Vec<Result<QueryResult>> {
        let handles: Vec<_> = queries
            .into_iter()
            .map(|q| {
                let providers = self.providers.clone();
                let index = self.capability_index.clone();
                tokio::spawn(async move {
                    let provider_indices = index
                        .get(&q.capability)
                        .ok_or(Error::NoProvider(q.capability))?;
                    let idx = provider_indices[0];
                    let provider = &providers[idx];
                    let start = Instant::now();
                    let mut result = provider.query(&q).await?;
                    result.latency_ms = start.elapsed().as_millis() as u64;
                    result.provider = provider.name().to_string();
                    Ok(result)
                })
            })
            .collect();

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(Error::Other(Box::new(e)))),
            }
        }
        results
    }

    /// Fan out health checks to all providers.
    pub async fn health(&self) -> SystemHealth {
        let handles: Vec<_> = self
            .providers
            .iter()
            .map(|p| {
                let p = p.clone();
                tokio::spawn(async move { p.health().await })
            })
            .collect();

        let mut provider_health = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(h) => provider_health.push(h),
                Err(_) => {} // join error — provider task panicked
            }
        }

        SystemHealth {
            town_id: self.config.town_id.clone(),
            providers: provider_health,
        }
    }

    /// All capabilities available across all registered providers.
    pub fn capabilities(&self) -> Vec<Capability> {
        self.capability_index.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Column, ColumnType, ProviderConfig, ProviderStatus, Value};

    /// A stub provider for testing.
    struct StubProvider {
        provider_name: String,
        caps: Vec<Capability>,
    }

    impl StubProvider {
        fn new(name: &str, caps: Vec<Capability>) -> Self {
            StubProvider {
                provider_name: name.to_string(),
                caps,
            }
        }
    }

    #[async_trait::async_trait]
    impl Provider for StubProvider {
        fn name(&self) -> &str {
            &self.provider_name
        }

        fn capabilities(&self) -> Vec<Capability> {
            self.caps.clone()
        }

        async fn health(&self) -> ProviderHealth {
            ProviderHealth {
                name: self.provider_name.clone(),
                status: ProviderStatus::Healthy,
                latency_ms: Some(1),
                message: None,
            }
        }

        async fn query(&self, _q: &Query) -> Result<QueryResult> {
            Ok(QueryResult {
                columns: vec![Column {
                    name: "id".into(),
                    col_type: ColumnType::Str,
                }],
                rows: vec![vec![Value::Str("test-1".into())]],
                total: Some(1),
                provider: self.provider_name.clone(),
                latency_ms: 0,
            })
        }

        async fn init(&mut self, _config: &ProviderConfig) -> Result<()> {
            Ok(())
        }
    }

    fn test_config() -> EngineConfig {
        EngineConfig {
            town_id: "test-town".into(),
        }
    }

    #[tokio::test]
    async fn engine_routes_query_to_provider() {
        let provider = Arc::new(StubProvider::new("stub", vec![Capability::Beads]));
        let engine = Engine::new(vec![provider], test_config());

        let q = Query {
            capability: Capability::Beads,
            filters: vec![],
            sort: None,
            limit: None,
            offset: None,
            fields: None,
        };
        let result = engine.query(&q).await.unwrap();
        assert_eq!(result.provider, "stub");
        assert_eq!(result.rows.len(), 1);
    }

    #[tokio::test]
    async fn engine_returns_error_for_unknown_capability() {
        let provider = Arc::new(StubProvider::new("stub", vec![Capability::Beads]));
        let engine = Engine::new(vec![provider], test_config());

        let q = Query {
            capability: Capability::Costs,
            filters: vec![],
            sort: None,
            limit: None,
            offset: None,
            fields: None,
        };
        let err = engine.query(&q).await.unwrap_err();
        assert!(matches!(err, Error::NoProvider(Capability::Costs)));
    }

    #[tokio::test]
    async fn engine_lists_capabilities() {
        let p1 = Arc::new(StubProvider::new("a", vec![Capability::Beads, Capability::Mail]));
        let p2 = Arc::new(StubProvider::new("b", vec![Capability::Costs]));
        let engine = Engine::new(vec![p1, p2], test_config());

        let caps = engine.capabilities();
        assert!(caps.contains(&Capability::Beads));
        assert!(caps.contains(&Capability::Mail));
        assert!(caps.contains(&Capability::Costs));
    }

    #[tokio::test]
    async fn engine_health_fans_out() {
        let p1 = Arc::new(StubProvider::new("a", vec![Capability::Beads]));
        let p2 = Arc::new(StubProvider::new("b", vec![Capability::Costs]));
        let engine = Engine::new(vec![p1, p2], test_config());

        let health = engine.health().await;
        assert_eq!(health.town_id, "test-town");
        assert_eq!(health.providers.len(), 2);
        assert!(health
            .providers
            .iter()
            .all(|h| h.status == ProviderStatus::Healthy));
    }

    #[tokio::test]
    async fn engine_query_many_parallel() {
        let provider = Arc::new(StubProvider::new(
            "stub",
            vec![Capability::Beads, Capability::Costs],
        ));
        let engine = Engine::new(vec![provider], test_config());

        let queries = vec![
            Query {
                capability: Capability::Beads,
                filters: vec![],
                sort: None,
                limit: None,
                offset: None,
                fields: None,
            },
            Query {
                capability: Capability::Costs,
                filters: vec![],
                sort: None,
                limit: None,
                offset: None,
                fields: None,
            },
        ];
        let results = engine.query_many(queries).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn engine_first_provider_wins() {
        let p1 = Arc::new(StubProvider::new("first", vec![Capability::Beads]));
        let p2 = Arc::new(StubProvider::new("second", vec![Capability::Beads]));
        let engine = Engine::new(vec![p1, p2], test_config());

        let q = Query {
            capability: Capability::Beads,
            filters: vec![],
            sort: None,
            limit: None,
            offset: None,
            fields: None,
        };
        let result = engine.query(&q).await.unwrap();
        assert_eq!(result.provider, "first");
    }

    #[tokio::test]
    async fn engine_no_providers_empty_capabilities() {
        let engine = Engine::new(vec![], test_config());
        assert!(engine.capabilities().is_empty());
    }
}
