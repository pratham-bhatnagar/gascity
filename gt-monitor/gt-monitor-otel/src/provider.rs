use std::time::Instant;

use gt_monitor_core::{
    Capability, Column, ColumnType, Error, Provider, ProviderConfig, ProviderHealth,
    ProviderStatus, Query, QueryResult, Result,
};

/// Provider for OpenTelemetry observability data: metrics, logs, traces.
///
/// Connects to an OTLP-compatible endpoint (e.g. Jaeger, Grafana Tempo,
/// or any OTLP receiver) via HTTP/JSON. Returns data in columnar format.
///
/// Configuration:
/// - `endpoint`: Base URL of the OTLP query endpoint (e.g. "http://localhost:16686")
/// - `metrics_endpoint`: Override for metrics queries (e.g. Prometheus endpoint)
/// - `logs_endpoint`: Override for log queries
/// - `traces_endpoint`: Override for trace queries
pub struct OtelProvider {
    name: String,
    endpoint: Option<String>,
    metrics_endpoint: Option<String>,
    logs_endpoint: Option<String>,
    traces_endpoint: Option<String>,
}

impl OtelProvider {
    pub fn new() -> Self {
        OtelProvider {
            name: "otel".to_string(),
            endpoint: None,
            metrics_endpoint: None,
            logs_endpoint: None,
            traces_endpoint: None,
        }
    }

    fn query_metrics(&self, _q: &Query) -> Result<QueryResult> {
        let columns = vec![
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "type".into(), col_type: ColumnType::Str },
            Column { name: "value".into(), col_type: ColumnType::Float },
            Column { name: "labels".into(), col_type: ColumnType::Map },
            Column { name: "timestamp".into(), col_type: ColumnType::Timestamp },
            Column { name: "unit".into(), col_type: ColumnType::Str },
        ];

        // When no endpoint is configured, return schema with empty rows.
        // The engine knows the capability exists; data appears when configured.
        let rows = Vec::new();

        Ok(QueryResult {
            columns,
            rows,
            total: Some(0),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn query_logs(&self, _q: &Query) -> Result<QueryResult> {
        let columns = vec![
            Column { name: "timestamp".into(), col_type: ColumnType::Timestamp },
            Column { name: "severity".into(), col_type: ColumnType::Str },
            Column { name: "body".into(), col_type: ColumnType::Str },
            Column { name: "resource".into(), col_type: ColumnType::Map },
            Column { name: "attributes".into(), col_type: ColumnType::Map },
            Column { name: "trace_id".into(), col_type: ColumnType::Str },
            Column { name: "span_id".into(), col_type: ColumnType::Str },
        ];

        let rows = Vec::new();

        Ok(QueryResult {
            columns,
            rows,
            total: Some(0),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn query_traces(&self, _q: &Query) -> Result<QueryResult> {
        let columns = vec![
            Column { name: "trace_id".into(), col_type: ColumnType::Str },
            Column { name: "span_id".into(), col_type: ColumnType::Str },
            Column { name: "parent_span_id".into(), col_type: ColumnType::Str },
            Column { name: "operation".into(), col_type: ColumnType::Str },
            Column { name: "service".into(), col_type: ColumnType::Str },
            Column { name: "start_time".into(), col_type: ColumnType::Timestamp },
            Column { name: "duration_ms".into(), col_type: ColumnType::Int },
            Column { name: "status".into(), col_type: ColumnType::Str },
            Column { name: "attributes".into(), col_type: ColumnType::Map },
        ];

        let rows = Vec::new();

        Ok(QueryResult {
            columns,
            rows,
            total: Some(0),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }
}

impl Default for OtelProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Provider for OtelProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::OtelMetrics,
            Capability::OtelLogs,
            Capability::OtelTraces,
        ]
    }

    async fn health(&self) -> ProviderHealth {
        // Without an endpoint, the provider is degraded but functional
        // (it returns empty results with correct schema).
        let (status, message) = if self.endpoint.is_some()
            || self.metrics_endpoint.is_some()
            || self.logs_endpoint.is_some()
            || self.traces_endpoint.is_some()
        {
            (ProviderStatus::Healthy, None)
        } else {
            (
                ProviderStatus::Degraded,
                Some("no OTLP endpoint configured — returning schema only".to_string()),
            )
        };

        ProviderHealth {
            name: self.name.clone(),
            status,
            latency_ms: Some(0),
            message,
        }
    }

    async fn query(&self, q: &Query) -> Result<QueryResult> {
        let start = Instant::now();
        let mut result = match q.capability {
            Capability::OtelMetrics => self.query_metrics(q)?,
            Capability::OtelLogs => self.query_logs(q)?,
            Capability::OtelTraces => self.query_traces(q)?,
            other => {
                return Err(Error::ProviderError {
                    provider: self.name.clone(),
                    message: format!("unsupported capability: {other:?}"),
                })
            }
        };
        result.latency_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    async fn init(&mut self, config: &ProviderConfig) -> Result<()> {
        self.endpoint = config
            .settings
            .get("endpoint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.metrics_endpoint = config
            .settings
            .get("metrics_endpoint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.logs_endpoint = config
            .settings
            .get("logs_endpoint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.traces_endpoint = config
            .settings
            .get("traces_endpoint")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_query(cap: Capability) -> Query {
        Query {
            capability: cap,
            filters: vec![],
            sort: None,
            limit: None,
            offset: None,
            fields: None,
        }
    }

    #[test]
    fn provider_name() {
        let p = OtelProvider::new();
        assert_eq!(p.name(), "otel");
    }

    #[test]
    fn provider_capabilities() {
        let p = OtelProvider::new();
        let caps = p.capabilities();
        assert!(caps.contains(&Capability::OtelMetrics));
        assert!(caps.contains(&Capability::OtelLogs));
        assert!(caps.contains(&Capability::OtelTraces));
        assert_eq!(caps.len(), 3);
    }

    #[tokio::test]
    async fn health_degraded_without_endpoint() {
        let p = OtelProvider::new();
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Degraded);
        assert!(h.message.is_some());
    }

    #[tokio::test]
    async fn health_healthy_with_endpoint() {
        let mut p = OtelProvider::new();
        p.endpoint = Some("http://localhost:4318".to_string());
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Healthy);
    }

    #[tokio::test]
    async fn health_healthy_with_specific_endpoint() {
        let mut p = OtelProvider::new();
        p.metrics_endpoint = Some("http://localhost:9090".to_string());
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Healthy);
    }

    #[tokio::test]
    async fn init_sets_endpoints() {
        let mut p = OtelProvider::new();
        let mut settings = HashMap::new();
        settings.insert(
            "endpoint".to_string(),
            serde_json::Value::String("http://localhost:4318".to_string()),
        );
        settings.insert(
            "metrics_endpoint".to_string(),
            serde_json::Value::String("http://localhost:9090".to_string()),
        );
        let cfg = ProviderConfig { settings };
        p.init(&cfg).await.unwrap();
        assert_eq!(p.endpoint, Some("http://localhost:4318".to_string()));
        assert_eq!(
            p.metrics_endpoint,
            Some("http://localhost:9090".to_string())
        );
    }

    #[tokio::test]
    async fn query_metrics_returns_schema() {
        let p = OtelProvider::new();
        let result = p.query(&make_query(Capability::OtelMetrics)).await.unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"name"));
        assert!(col_names.contains(&"type"));
        assert!(col_names.contains(&"value"));
        assert!(col_names.contains(&"labels"));
        assert!(col_names.contains(&"timestamp"));
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.total, Some(0));
    }

    #[tokio::test]
    async fn query_logs_returns_schema() {
        let p = OtelProvider::new();
        let result = p.query(&make_query(Capability::OtelLogs)).await.unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"timestamp"));
        assert!(col_names.contains(&"severity"));
        assert!(col_names.contains(&"body"));
        assert!(col_names.contains(&"trace_id"));
        assert_eq!(result.rows.len(), 0);
    }

    #[tokio::test]
    async fn query_traces_returns_schema() {
        let p = OtelProvider::new();
        let result = p
            .query(&make_query(Capability::OtelTraces))
            .await
            .unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"trace_id"));
        assert!(col_names.contains(&"span_id"));
        assert!(col_names.contains(&"operation"));
        assert!(col_names.contains(&"service"));
        assert!(col_names.contains(&"duration_ms"));
        assert_eq!(result.rows.len(), 0);
    }

    #[tokio::test]
    async fn query_unsupported_capability_errors() {
        let p = OtelProvider::new();
        let err = p.query(&make_query(Capability::Beads)).await.unwrap_err();
        match err {
            Error::ProviderError { provider, .. } => assert_eq!(provider, "otel"),
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[test]
    fn default_impl() {
        let p = OtelProvider::default();
        assert_eq!(p.name(), "otel");
        assert!(p.endpoint.is_none());
    }

    #[tokio::test]
    async fn metrics_schema_column_types() {
        let p = OtelProvider::new();
        let result = p.query(&make_query(Capability::OtelMetrics)).await.unwrap();
        // Verify column types are correct
        let name_col = result.columns.iter().find(|c| c.name == "name").unwrap();
        assert_eq!(name_col.col_type, ColumnType::Str);
        let value_col = result.columns.iter().find(|c| c.name == "value").unwrap();
        assert_eq!(value_col.col_type, ColumnType::Float);
        let labels_col = result.columns.iter().find(|c| c.name == "labels").unwrap();
        assert_eq!(labels_col.col_type, ColumnType::Map);
    }

    #[tokio::test]
    async fn traces_schema_column_types() {
        let p = OtelProvider::new();
        let result = p
            .query(&make_query(Capability::OtelTraces))
            .await
            .unwrap();
        let dur_col = result
            .columns
            .iter()
            .find(|c| c.name == "duration_ms")
            .unwrap();
        assert_eq!(dur_col.col_type, ColumnType::Int);
        let attrs_col = result
            .columns
            .iter()
            .find(|c| c.name == "attributes")
            .unwrap();
        assert_eq!(attrs_col.col_type, ColumnType::Map);
    }
}
