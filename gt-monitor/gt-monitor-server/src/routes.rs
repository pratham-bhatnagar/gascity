use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use gt_monitor_core::{Capability, Query, QueryResult};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::AppState;

/// Build the axum Router with all /v1/* routes and middleware.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/query", post(query))
        .route("/v1/query/batch", post(query_batch))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

// --- Response types ---

#[derive(Serialize)]
struct HealthResponse {
    town_id: String,
    status: String,
    providers: Vec<ProviderHealthResponse>,
}

#[derive(Serialize)]
struct ProviderHealthResponse {
    name: String,
    status: String,
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize)]
struct CapabilitiesResponse {
    capabilities: Vec<Capability>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct BatchQueryRequest {
    queries: Vec<Query>,
}

#[derive(Serialize)]
struct BatchQueryResponse {
    results: Vec<BatchQueryItem>,
}

#[derive(Serialize)]
struct BatchQueryItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<QueryResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// --- Handlers ---

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.engine.health().await;

    let overall_status = if health.providers.iter().all(|p| {
        p.status == gt_monitor_core::ProviderStatus::Healthy
    }) {
        "healthy"
    } else if health.providers.iter().any(|p| {
        p.status == gt_monitor_core::ProviderStatus::Unavailable
    }) {
        "degraded"
    } else {
        "degraded"
    };

    let providers: Vec<ProviderHealthResponse> = health
        .providers
        .into_iter()
        .map(|p| ProviderHealthResponse {
            name: p.name,
            status: format!("{:?}", p.status).to_lowercase(),
            latency_ms: p.latency_ms,
            message: p.message,
        })
        .collect();

    Json(HealthResponse {
        town_id: health.town_id,
        status: overall_status.to_string(),
        providers,
    })
}

async fn capabilities(State(state): State<AppState>) -> impl IntoResponse {
    let caps = state.engine.capabilities();
    Json(CapabilitiesResponse {
        capabilities: caps,
    })
}

async fn query(
    State(state): State<AppState>,
    Json(q): Json<Query>,
) -> std::result::Result<Json<QueryResult>, (StatusCode, Json<ErrorResponse>)> {
    state.engine.query(&q).await.map(Json).map_err(|e| {
        let (status, msg) = error_to_status(&e);
        (status, Json(ErrorResponse { error: msg }))
    })
}

async fn query_batch(
    State(state): State<AppState>,
    Json(req): Json<BatchQueryRequest>,
) -> impl IntoResponse {
    let results = state.engine.query_many(req.queries).await;

    let items: Vec<BatchQueryItem> = results
        .into_iter()
        .map(|r| match r {
            Ok(qr) => BatchQueryItem {
                data: Some(qr),
                error: None,
            },
            Err(e) => BatchQueryItem {
                data: None,
                error: Some(e.to_string()),
            },
        })
        .collect();

    Json(BatchQueryResponse { results: items })
}

fn error_to_status(e: &gt_monitor_core::Error) -> (StatusCode, String) {
    match e {
        gt_monitor_core::Error::NoProvider(_) => {
            (StatusCode::NOT_FOUND, e.to_string())
        }
        gt_monitor_core::Error::InvalidQuery(_) => {
            (StatusCode::BAD_REQUEST, e.to_string())
        }
        gt_monitor_core::Error::NotInitialized { .. } => {
            (StatusCode::SERVICE_UNAVAILABLE, e.to_string())
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use gt_monitor_core::{
        Column, ColumnType, Engine, EngineConfig, Provider, ProviderConfig, ProviderHealth,
        ProviderStatus, Value,
    };
    use http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    /// Stub provider for HTTP handler tests.
    struct StubProvider {
        caps: Vec<Capability>,
    }

    impl StubProvider {
        fn new(caps: Vec<Capability>) -> Self {
            StubProvider { caps }
        }
    }

    #[async_trait::async_trait]
    impl Provider for StubProvider {
        fn name(&self) -> &str {
            "stub"
        }

        fn capabilities(&self) -> Vec<Capability> {
            self.caps.clone()
        }

        async fn health(&self) -> ProviderHealth {
            ProviderHealth {
                name: "stub".into(),
                status: ProviderStatus::Healthy,
                latency_ms: Some(1),
                message: None,
            }
        }

        async fn query(&self, _q: &Query) -> gt_monitor_core::Result<QueryResult> {
            Ok(QueryResult {
                columns: vec![Column {
                    name: "id".into(),
                    col_type: ColumnType::Str,
                }],
                rows: vec![vec![Value::Str("test-1".into())]],
                total: Some(1),
                provider: "stub".into(),
                latency_ms: 0,
            })
        }

        async fn init(&mut self, _config: &ProviderConfig) -> gt_monitor_core::Result<()> {
            Ok(())
        }
    }

    fn test_app() -> Router {
        let provider = Arc::new(StubProvider::new(vec![
            Capability::Beads,
            Capability::Costs,
        ]));
        let engine = Engine::new(
            vec![provider],
            EngineConfig {
                town_id: "test-town".into(),
            },
        );
        router(AppState::new(engine))
    }

    fn empty_app() -> Router {
        let engine = Engine::new(
            vec![],
            EngineConfig {
                town_id: "empty-town".into(),
            },
        );
        router(AppState::new(engine))
    }

    async fn body_to_string(body: Body) -> String {
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn health_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .uri("/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["town_id"], "test-town");
        assert_eq!(json["status"], "healthy");
        assert_eq!(json["providers"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn health_empty_engine() {
        let app = empty_app();
        let req = Request::builder()
            .uri("/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["town_id"], "empty-town");
        // No providers → healthy (vacuously true)
        assert_eq!(json["status"], "healthy");
    }

    #[tokio::test]
    async fn capabilities_returns_list() {
        let app = test_app();
        let req = Request::builder()
            .uri("/v1/capabilities")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let caps = json["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
    }

    #[tokio::test]
    async fn query_returns_result() {
        let app = test_app();
        let query_json = r#"{"capability":"beads"}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/query")
            .header("content-type", "application/json")
            .body(Body::from(query_json))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["provider"], "stub");
        assert_eq!(json["rows"].as_array().unwrap().len(), 1);
        assert_eq!(json["columns"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn query_unknown_capability_returns_404() {
        let app = test_app();
        let query_json = r#"{"capability":"system_cpu"}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/query")
            .header("content-type", "application/json")
            .body(Body::from(query_json))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("no provider"));
    }

    #[tokio::test]
    async fn query_invalid_json_returns_error() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/query")
            .header("content-type", "application/json")
            .body(Body::from("not json"))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // axum returns 400 for JSON parse errors
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn query_with_filters() {
        let app = test_app();
        let query_json = r#"{
            "capability": "beads",
            "filters": [{"field": "status", "op": "eq", "value": "open"}],
            "limit": 10
        }"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/query")
            .header("content-type", "application/json")
            .body(Body::from(query_json))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn batch_query_returns_results() {
        let app = test_app();
        let batch_json = r#"{
            "queries": [
                {"capability": "beads"},
                {"capability": "costs"}
            ]
        }"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/query/batch")
            .header("content-type", "application/json")
            .body(Body::from(batch_json))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let results = json["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        // Both should have data, no errors
        for item in results {
            assert!(item["data"].is_object());
            assert!(item["error"].is_null());
        }
    }

    #[tokio::test]
    async fn batch_query_partial_failure() {
        let app = test_app();
        let batch_json = r#"{
            "queries": [
                {"capability": "beads"},
                {"capability": "system_cpu"}
            ]
        }"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/query/batch")
            .header("content-type", "application/json")
            .body(Body::from(batch_json))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let results = json["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        // First succeeds
        assert!(results[0]["data"].is_object());
        assert!(results[0]["error"].is_null());
        // Second fails (no provider)
        assert!(results[1]["data"].is_null());
        assert!(results[1]["error"].as_str().unwrap().contains("no provider"));
    }

    #[tokio::test]
    async fn nonexistent_route_returns_404() {
        let app = test_app();
        let req = Request::builder()
            .uri("/v1/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn query_get_method_not_allowed() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/query")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn batch_empty_queries() {
        let app = test_app();
        let batch_json = r#"{"queries": []}"#;
        let req = Request::builder()
            .method("POST")
            .uri("/v1/query/batch")
            .header("content-type", "application/json")
            .body(Body::from(batch_json))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_to_string(resp.into_body()).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["results"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn cors_headers_present() {
        let app = test_app();
        let req = Request::builder()
            .method("OPTIONS")
            .uri("/v1/health")
            .header("origin", "http://localhost:3000")
            .header("access-control-request-method", "GET")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.headers().contains_key("access-control-allow-origin"));
    }
}
