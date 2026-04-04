use std::collections::HashMap;

use gt_monitor_core::{
    Capability, Filter, FilterOp, Provider, ProviderConfig, ProviderStatus, Query, Value,
};
use gt_monitor_gitea::GiteaProvider;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn init_provider(server: &MockServer) -> GiteaProvider {
    let mut p = GiteaProvider::new();
    let mut settings = HashMap::new();
    settings.insert("url".into(), serde_json::json!(server.uri()));
    settings.insert("token".into(), serde_json::json!("test-token"));
    p.init(&ProviderConfig { settings }).await.unwrap();
    p
}

fn owner_repo_filters() -> Vec<Filter> {
    vec![
        Filter {
            field: "owner".into(),
            op: FilterOp::Eq,
            value: Value::Str("gastownhall".into()),
        },
        Filter {
            field: "repo".into(),
            op: FilterOp::Eq,
            value: Value::Str("gascity".into()),
        },
    ]
}

#[tokio::test]
async fn health_check_returns_healthy() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "1.22.0"
        })))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let health = p.health().await;

    assert_eq!(health.status, ProviderStatus::Healthy);
    assert_eq!(health.name, "gitea");
    assert!(health.message.unwrap().contains("1.22.0"));
    assert!(health.latency_ms.is_some());
}

#[tokio::test]
async fn health_check_returns_unavailable_on_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/version"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let health = p.health().await;

    assert_eq!(health.status, ProviderStatus::Unavailable);
}

#[tokio::test]
async fn query_repos() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/repos/search"))
        .and(query_param("page", "1"))
        .and(query_param("limit", "30"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 1,
                "full_name": "gastownhall/gascity",
                "name": "gascity",
                "description": "Gas City SDK",
                "private": false,
                "fork": false,
                "archived": false,
                "stars_count": 42,
                "forks_count": 5,
                "open_issues_count": 10,
                "default_branch": "main",
                "created_at": "2025-01-01T00:00:00Z",
                "updated_at": "2026-04-01T00:00:00Z",
                "owner": {
                    "id": 1,
                    "login": "gastownhall",
                    "full_name": "Gas Town Hall",
                    "email": null,
                    "avatar_url": null,
                    "created": null
                }
            }
        ])))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let q = Query {
        capability: Capability::Repos,
        filters: vec![],
        sort: None,
        limit: None,
        offset: None,
        fields: None,
    };

    let result = p.query(&q).await.unwrap();
    assert_eq!(result.provider, "gitea");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.columns.len(), 14);
    assert_eq!(result.rows[0][1], Value::Str("gastownhall/gascity".into()));
}

#[tokio::test]
async fn query_issues() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/repos/gastownhall/gascity/issues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 100,
                "number": 42,
                "title": "Fix the thing",
                "body": "It's broken",
                "state": "open",
                "user": { "id": 1, "login": "alice", "full_name": null, "email": null, "avatar_url": null, "created": null },
                "labels": [{ "name": "bug" }],
                "created_at": "2026-04-01T00:00:00Z",
                "updated_at": "2026-04-02T00:00:00Z",
                "closed_at": null
            }
        ])))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let q = Query {
        capability: Capability::Issues,
        filters: owner_repo_filters(),
        sort: None,
        limit: None,
        offset: None,
        fields: None,
    };

    let result = p.query(&q).await.unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][2], Value::Str("Fix the thing".into()));
    assert_eq!(result.rows[0][4], Value::Str("open".into()));
    assert_eq!(result.rows[0][5], Value::Str("alice".into()));
    assert_eq!(result.rows[0][6], Value::Str("bug".into()));
}

#[tokio::test]
async fn query_pull_requests() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/repos/gastownhall/gascity/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 200,
                "number": 7,
                "title": "Add feature",
                "body": "New stuff",
                "state": "open",
                "user": { "id": 2, "login": "bob", "full_name": null, "email": null, "avatar_url": null, "created": null },
                "head": { "ref": "feature-branch" },
                "base": { "ref": "main" },
                "merged": false,
                "created_at": "2026-04-03T00:00:00Z",
                "updated_at": "2026-04-03T12:00:00Z",
                "merged_at": null
            }
        ])))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let q = Query {
        capability: Capability::PullRequests,
        filters: owner_repo_filters(),
        sort: None,
        limit: None,
        offset: None,
        fields: None,
    };

    let result = p.query(&q).await.unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][2], Value::Str("Add feature".into()));
    assert_eq!(result.rows[0][6], Value::Str("feature-branch".into()));
    assert_eq!(result.rows[0][7], Value::Str("main".into()));
    assert_eq!(result.rows[0][8], Value::Bool(false));
}

#[tokio::test]
async fn query_commits() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/repos/gastownhall/gascity/git/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "sha": "abc123",
                "commit": {
                    "message": "fix: something",
                    "author": {
                        "name": "Alice",
                        "email": "alice@example.com",
                        "date": "2026-04-04T08:00:00Z"
                    }
                },
                "author": { "id": 1, "login": "alice", "full_name": null, "email": null, "avatar_url": null, "created": null }
            }
        ])))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let q = Query {
        capability: Capability::Commits,
        filters: owner_repo_filters(),
        sort: None,
        limit: None,
        offset: None,
        fields: None,
    };

    let result = p.query(&q).await.unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][0], Value::Str("abc123".into()));
    assert_eq!(result.rows[0][1], Value::Str("fix: something".into()));
    assert_eq!(result.rows[0][2], Value::Str("Alice".into()));
    assert_eq!(result.rows[0][4], Value::Str("alice".into()));
}

#[tokio::test]
async fn query_users() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/users/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": 1,
                    "login": "alice",
                    "full_name": "Alice A",
                    "email": "alice@example.com",
                    "avatar_url": "https://example.com/alice.png",
                    "created": "2025-01-01T00:00:00Z"
                },
                {
                    "id": 2,
                    "login": "bob",
                    "full_name": null,
                    "email": null,
                    "avatar_url": null,
                    "created": null
                }
            ]
        })))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let q = Query {
        capability: Capability::Users,
        filters: vec![],
        sort: None,
        limit: None,
        offset: None,
        fields: None,
    };

    let result = p.query(&q).await.unwrap();
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0][1], Value::Str("alice".into()));
    assert_eq!(result.rows[1][1], Value::Str("bob".into()));
    assert_eq!(result.rows[1][2], Value::Null);
}

#[tokio::test]
async fn field_projection_works() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/repos/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 1,
                "full_name": "gastownhall/gascity",
                "name": "gascity",
                "description": null,
                "private": false,
                "fork": false,
                "archived": false,
                "stars_count": 42,
                "forks_count": 5,
                "open_issues_count": 10,
                "default_branch": "main",
                "created_at": "2025-01-01T00:00:00Z",
                "updated_at": "2026-04-01T00:00:00Z",
                "owner": { "id": 1, "login": "gastownhall", "full_name": null, "email": null, "avatar_url": null, "created": null }
            }
        ])))
        .mount(&server)
        .await;

    let p = init_provider(&server).await;
    let q = Query {
        capability: Capability::Repos,
        filters: vec![],
        sort: None,
        limit: None,
        offset: None,
        fields: Some(vec!["full_name".into(), "stars_count".into()]),
    };

    let result = p.query(&q).await.unwrap();
    assert_eq!(result.columns.len(), 2);
    assert_eq!(result.columns[0].name, "full_name");
    assert_eq!(result.columns[1].name, "stars_count");
    assert_eq!(result.rows[0].len(), 2);
}

#[tokio::test]
async fn unsupported_capability_returns_error() {
    let server = MockServer::start().await;
    let p = init_provider(&server).await;

    let q = Query {
        capability: Capability::Beads,
        filters: vec![],
        sort: None,
        limit: None,
        offset: None,
        fields: None,
    };

    let err = p.query(&q).await.unwrap_err();
    assert!(err.to_string().contains("does not support"));
}
