use std::time::Instant;

use gt_monitor_core::{
    Capability, Column, ColumnType, Error, Filter, FilterOp, ProviderConfig, ProviderHealth,
    ProviderStatus, Query, QueryResult, Result, Value,
};

use crate::client::{
    GiteaClient, GiteaCommit, GiteaIssue, GiteaPull, GiteaRepo, GiteaUser,
};

/// Gitea REST API provider for gt-monitor.
///
/// Supports: Repos, Issues, PullRequests, Commits, Users.
pub struct GiteaProvider {
    client: Option<GiteaClient>,
}

impl GiteaProvider {
    pub fn new() -> Self {
        GiteaProvider { client: None }
    }

    fn client(&self) -> Result<&GiteaClient> {
        self.client.as_ref().ok_or(Error::NotInitialized {
            provider: "gitea".into(),
        })
    }

    /// Extract owner/repo from query filters. Looks for "owner" and "repo" Eq filters.
    fn extract_owner_repo(filters: &[Filter]) -> (Option<&str>, Option<&str>) {
        let mut owner = None;
        let mut repo = None;
        for f in filters {
            if f.op == FilterOp::Eq {
                if let Value::Str(ref s) = f.value {
                    match f.field.as_str() {
                        "owner" => owner = Some(s.as_str()),
                        "repo" => repo = Some(s.as_str()),
                        _ => {}
                    }
                }
            }
        }
        (owner, repo)
    }

    /// Extract string filter value for a given field name.
    fn filter_str<'a>(filters: &'a [Filter], field: &str) -> Option<&'a str> {
        filters.iter().find_map(|f| {
            if f.field == field && f.op == FilterOp::Eq {
                if let Value::Str(ref s) = f.value {
                    Some(s.as_str())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    fn page_params(q: &Query) -> (u32, u32) {
        let limit = q.limit.unwrap_or(30).min(50) as u32;
        let page = match q.offset {
            Some(off) if limit > 0 => (off as u32 / limit) + 1,
            _ => 1,
        };
        (page, limit)
    }

    async fn query_repos(&self, q: &Query) -> Result<QueryResult> {
        let client = self.client()?;
        let (page, limit) = Self::page_params(q);
        let sort = q.sort.as_ref().map(|s| s.field.as_str());
        let order = q.sort.as_ref().map(|s| match s.dir {
            gt_monitor_core::SortDir::Asc => "asc",
            gt_monitor_core::SortDir::Desc => "desc",
        });

        let repos = client
            .list_repos(page, limit, sort, order)
            .await
            .map_err(|e| Error::ProviderError {
                provider: "gitea".into(),
                message: e.to_string(),
            })?;

        Ok(repos_to_result(repos))
    }

    async fn query_issues(&self, q: &Query) -> Result<QueryResult> {
        let client = self.client()?;
        let (owner, repo) = Self::extract_owner_repo(&q.filters);
        let (owner, repo) = match (owner, repo) {
            (Some(o), Some(r)) => (o, r),
            _ => {
                return Err(Error::InvalidQuery(
                    "issues query requires 'owner' and 'repo' filters".into(),
                ))
            }
        };
        let state = Self::filter_str(&q.filters, "state");
        let issue_type = Self::filter_str(&q.filters, "type");
        let (page, limit) = Self::page_params(q);

        let issues = client
            .list_issues(owner, repo, state, issue_type, page, limit)
            .await
            .map_err(|e| Error::ProviderError {
                provider: "gitea".into(),
                message: e.to_string(),
            })?;

        Ok(issues_to_result(issues))
    }

    async fn query_pulls(&self, q: &Query) -> Result<QueryResult> {
        let client = self.client()?;
        let (owner, repo) = Self::extract_owner_repo(&q.filters);
        let (owner, repo) = match (owner, repo) {
            (Some(o), Some(r)) => (o, r),
            _ => {
                return Err(Error::InvalidQuery(
                    "pull_requests query requires 'owner' and 'repo' filters".into(),
                ))
            }
        };
        let state = Self::filter_str(&q.filters, "state");
        let (page, limit) = Self::page_params(q);

        let pulls = client
            .list_pulls(owner, repo, state, page, limit)
            .await
            .map_err(|e| Error::ProviderError {
                provider: "gitea".into(),
                message: e.to_string(),
            })?;

        Ok(pulls_to_result(pulls))
    }

    async fn query_commits(&self, q: &Query) -> Result<QueryResult> {
        let client = self.client()?;
        let (owner, repo) = Self::extract_owner_repo(&q.filters);
        let (owner, repo) = match (owner, repo) {
            (Some(o), Some(r)) => (o, r),
            _ => {
                return Err(Error::InvalidQuery(
                    "commits query requires 'owner' and 'repo' filters".into(),
                ))
            }
        };
        let sha = Self::filter_str(&q.filters, "sha");
        let (page, limit) = Self::page_params(q);

        let commits = client
            .list_commits(owner, repo, sha, page, limit)
            .await
            .map_err(|e| Error::ProviderError {
                provider: "gitea".into(),
                message: e.to_string(),
            })?;

        Ok(commits_to_result(commits))
    }

    async fn query_users(&self, q: &Query) -> Result<QueryResult> {
        let client = self.client()?;
        let (page, limit) = Self::page_params(q);

        let users = client
            .list_users(page, limit)
            .await
            .map_err(|e| Error::ProviderError {
                provider: "gitea".into(),
                message: e.to_string(),
            })?;

        Ok(users_to_result(users))
    }
}

impl Default for GiteaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl gt_monitor_core::Provider for GiteaProvider {
    fn name(&self) -> &str {
        "gitea"
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Repos,
            Capability::Issues,
            Capability::PullRequests,
            Capability::Commits,
            Capability::Users,
        ]
    }

    async fn health(&self) -> ProviderHealth {
        let client = match self.client.as_ref() {
            Some(c) => c,
            None => {
                return ProviderHealth {
                    name: "gitea".into(),
                    status: ProviderStatus::Unavailable,
                    latency_ms: None,
                    message: Some("not initialized".into()),
                }
            }
        };

        match client.version().await {
            Ok((version, ms)) => ProviderHealth {
                name: "gitea".into(),
                status: ProviderStatus::Healthy,
                latency_ms: Some(ms),
                message: Some(format!("v{version}")),
            },
            Err(e) => ProviderHealth {
                name: "gitea".into(),
                status: ProviderStatus::Unavailable,
                latency_ms: None,
                message: Some(e.to_string()),
            },
        }
    }

    async fn query(&self, q: &Query) -> Result<QueryResult> {
        let start = Instant::now();
        let mut result = match q.capability {
            Capability::Repos => self.query_repos(q).await?,
            Capability::Issues => self.query_issues(q).await?,
            Capability::PullRequests => self.query_pulls(q).await?,
            Capability::Commits => self.query_commits(q).await?,
            Capability::Users => self.query_users(q).await?,
            other => {
                return Err(Error::InvalidQuery(format!(
                    "gitea provider does not support capability {other:?}"
                )))
            }
        };

        // Apply field projection if requested.
        if let Some(ref fields) = q.fields {
            result = project_fields(result, fields);
        }

        result.latency_ms = start.elapsed().as_millis() as u64;
        result.provider = "gitea".into();
        Ok(result)
    }

    async fn init(&mut self, config: &ProviderConfig) -> Result<()> {
        let url = config
            .settings
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidQuery("gitea provider requires 'url' config".into()))?;

        let token = config
            .settings
            .get("token")
            .and_then(|v| v.as_str());

        self.client = Some(GiteaClient::new(url, token).map_err(|e| Error::ProviderError {
            provider: "gitea".into(),
            message: e.to_string(),
        })?);

        Ok(())
    }
}

// ----- Conversion helpers -----

fn val_str(s: &str) -> Value {
    Value::Str(s.to_string())
}

fn val_opt_str(s: &Option<String>) -> Value {
    match s {
        Some(s) => Value::Str(s.clone()),
        None => Value::Null,
    }
}

fn val_ts(dt: &chrono::DateTime<chrono::Utc>) -> Value {
    Value::Timestamp(*dt)
}

fn val_opt_ts(dt: &Option<chrono::DateTime<chrono::Utc>>) -> Value {
    match dt {
        Some(dt) => Value::Timestamp(*dt),
        None => Value::Null,
    }
}

fn repos_to_result(repos: Vec<GiteaRepo>) -> QueryResult {
    let columns = vec![
        Column { name: "id".into(), col_type: ColumnType::Int },
        Column { name: "full_name".into(), col_type: ColumnType::Str },
        Column { name: "name".into(), col_type: ColumnType::Str },
        Column { name: "description".into(), col_type: ColumnType::Str },
        Column { name: "private".into(), col_type: ColumnType::Bool },
        Column { name: "fork".into(), col_type: ColumnType::Bool },
        Column { name: "archived".into(), col_type: ColumnType::Bool },
        Column { name: "stars_count".into(), col_type: ColumnType::Int },
        Column { name: "forks_count".into(), col_type: ColumnType::Int },
        Column { name: "open_issues_count".into(), col_type: ColumnType::Int },
        Column { name: "default_branch".into(), col_type: ColumnType::Str },
        Column { name: "owner".into(), col_type: ColumnType::Str },
        Column { name: "created_at".into(), col_type: ColumnType::Timestamp },
        Column { name: "updated_at".into(), col_type: ColumnType::Timestamp },
    ];

    let total = repos.len();
    let rows: Vec<Vec<Value>> = repos
        .into_iter()
        .map(|r| {
            vec![
                Value::Int(r.id),
                val_str(&r.full_name),
                val_str(&r.name),
                val_opt_str(&r.description),
                Value::Bool(r.private),
                Value::Bool(r.fork),
                Value::Bool(r.archived),
                Value::Int(r.stars_count),
                Value::Int(r.forks_count),
                Value::Int(r.open_issues_count),
                val_opt_str(&r.default_branch),
                Value::Str(r.owner.map(|u| u.login).unwrap_or_default()),
                val_ts(&r.created_at),
                val_ts(&r.updated_at),
            ]
        })
        .collect();

    QueryResult {
        columns,
        rows,
        total: Some(total),
        provider: "gitea".into(),
        latency_ms: 0,
    }
}

fn issues_to_result(issues: Vec<GiteaIssue>) -> QueryResult {
    let columns = vec![
        Column { name: "id".into(), col_type: ColumnType::Int },
        Column { name: "number".into(), col_type: ColumnType::Int },
        Column { name: "title".into(), col_type: ColumnType::Str },
        Column { name: "body".into(), col_type: ColumnType::Str },
        Column { name: "state".into(), col_type: ColumnType::Str },
        Column { name: "author".into(), col_type: ColumnType::Str },
        Column { name: "labels".into(), col_type: ColumnType::Str },
        Column { name: "created_at".into(), col_type: ColumnType::Timestamp },
        Column { name: "updated_at".into(), col_type: ColumnType::Timestamp },
        Column { name: "closed_at".into(), col_type: ColumnType::Timestamp },
    ];

    let total = issues.len();
    let rows: Vec<Vec<Value>> = issues
        .into_iter()
        .map(|i| {
            let labels_str = i
                .labels
                .as_ref()
                .map(|ls| {
                    ls.iter()
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            vec![
                Value::Int(i.id),
                Value::Int(i.number),
                val_str(&i.title),
                val_opt_str(&i.body),
                val_str(&i.state),
                Value::Str(i.user.map(|u| u.login).unwrap_or_default()),
                val_str(&labels_str),
                val_ts(&i.created_at),
                val_ts(&i.updated_at),
                val_opt_ts(&i.closed_at),
            ]
        })
        .collect();

    QueryResult {
        columns,
        rows,
        total: Some(total),
        provider: "gitea".into(),
        latency_ms: 0,
    }
}

fn pulls_to_result(pulls: Vec<GiteaPull>) -> QueryResult {
    let columns = vec![
        Column { name: "id".into(), col_type: ColumnType::Int },
        Column { name: "number".into(), col_type: ColumnType::Int },
        Column { name: "title".into(), col_type: ColumnType::Str },
        Column { name: "body".into(), col_type: ColumnType::Str },
        Column { name: "state".into(), col_type: ColumnType::Str },
        Column { name: "author".into(), col_type: ColumnType::Str },
        Column { name: "head_ref".into(), col_type: ColumnType::Str },
        Column { name: "base_ref".into(), col_type: ColumnType::Str },
        Column { name: "merged".into(), col_type: ColumnType::Bool },
        Column { name: "created_at".into(), col_type: ColumnType::Timestamp },
        Column { name: "updated_at".into(), col_type: ColumnType::Timestamp },
        Column { name: "merged_at".into(), col_type: ColumnType::Timestamp },
    ];

    let total = pulls.len();
    let rows: Vec<Vec<Value>> = pulls
        .into_iter()
        .map(|p| {
            let head_ref = p
                .head
                .as_ref()
                .and_then(|h| h.ref_name.clone())
                .unwrap_or_default();
            let base_ref = p
                .base
                .as_ref()
                .and_then(|b| b.ref_name.clone())
                .unwrap_or_default();
            vec![
                Value::Int(p.id),
                Value::Int(p.number),
                val_str(&p.title),
                val_opt_str(&p.body),
                val_str(&p.state),
                Value::Str(p.user.map(|u| u.login).unwrap_or_default()),
                val_str(&head_ref),
                val_str(&base_ref),
                Value::Bool(p.merged.unwrap_or(false)),
                val_ts(&p.created_at),
                val_ts(&p.updated_at),
                val_opt_ts(&p.merged_at),
            ]
        })
        .collect();

    QueryResult {
        columns,
        rows,
        total: Some(total),
        provider: "gitea".into(),
        latency_ms: 0,
    }
}

fn commits_to_result(commits: Vec<GiteaCommit>) -> QueryResult {
    let columns = vec![
        Column { name: "sha".into(), col_type: ColumnType::Str },
        Column { name: "message".into(), col_type: ColumnType::Str },
        Column { name: "author_name".into(), col_type: ColumnType::Str },
        Column { name: "author_email".into(), col_type: ColumnType::Str },
        Column { name: "author_login".into(), col_type: ColumnType::Str },
        Column { name: "date".into(), col_type: ColumnType::Timestamp },
    ];

    let total = commits.len();
    let rows: Vec<Vec<Value>> = commits
        .into_iter()
        .map(|c| {
            let detail = c.commit.as_ref();
            let author = detail.and_then(|d| d.author.as_ref());
            vec![
                val_str(&c.sha),
                Value::Str(
                    detail
                        .and_then(|d| d.message.clone())
                        .unwrap_or_default(),
                ),
                Value::Str(
                    author
                        .and_then(|a| a.name.clone())
                        .unwrap_or_default(),
                ),
                Value::Str(
                    author
                        .and_then(|a| a.email.clone())
                        .unwrap_or_default(),
                ),
                Value::Str(c.author.map(|u| u.login).unwrap_or_default()),
                val_opt_ts(&author.and_then(|a| a.date)),
            ]
        })
        .collect();

    QueryResult {
        columns,
        rows,
        total: Some(total),
        provider: "gitea".into(),
        latency_ms: 0,
    }
}

fn users_to_result(users: Vec<GiteaUser>) -> QueryResult {
    let columns = vec![
        Column { name: "id".into(), col_type: ColumnType::Int },
        Column { name: "login".into(), col_type: ColumnType::Str },
        Column { name: "full_name".into(), col_type: ColumnType::Str },
        Column { name: "email".into(), col_type: ColumnType::Str },
        Column { name: "avatar_url".into(), col_type: ColumnType::Str },
        Column { name: "created".into(), col_type: ColumnType::Timestamp },
    ];

    let total = users.len();
    let rows: Vec<Vec<Value>> = users
        .into_iter()
        .map(|u| {
            vec![
                Value::Int(u.id),
                val_str(&u.login),
                val_opt_str(&u.full_name),
                val_opt_str(&u.email),
                val_opt_str(&u.avatar_url),
                val_opt_ts(&u.created),
            ]
        })
        .collect();

    QueryResult {
        columns,
        rows,
        total: Some(total),
        provider: "gitea".into(),
        latency_ms: 0,
    }
}

/// Reduce a QueryResult to only the requested field columns.
fn project_fields(result: QueryResult, fields: &[String]) -> QueryResult {
    let indices: Vec<usize> = fields
        .iter()
        .filter_map(|f| result.columns.iter().position(|c| c.name == *f))
        .collect();

    let columns: Vec<Column> = indices.iter().map(|&i| result.columns[i].clone()).collect();
    let rows: Vec<Vec<Value>> = result
        .rows
        .into_iter()
        .map(|row| indices.iter().map(|&i| row[i].clone()).collect())
        .collect();

    QueryResult {
        columns,
        rows,
        total: result.total,
        provider: result.provider,
        latency_ms: result.latency_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gt_monitor_core::{Filter, FilterOp, Provider as _, Value};

    #[test]
    fn capabilities_include_all_five() {
        let p = GiteaProvider::new();
        let caps = p.capabilities();
        assert!(caps.contains(&Capability::Repos));
        assert!(caps.contains(&Capability::Issues));
        assert!(caps.contains(&Capability::PullRequests));
        assert!(caps.contains(&Capability::Commits));
        assert!(caps.contains(&Capability::Users));
        assert_eq!(caps.len(), 5);
    }

    #[test]
    fn name_is_gitea() {
        let p = GiteaProvider::new();
        assert_eq!(p.name(), "gitea");
    }

    #[tokio::test]
    async fn health_returns_unavailable_when_not_initialized() {
        let p = GiteaProvider::new();
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Unavailable);
        assert_eq!(h.name, "gitea");
    }

    #[tokio::test]
    async fn query_before_init_returns_not_initialized() {
        let p = GiteaProvider::new();
        let q = Query {
            capability: Capability::Repos,
            filters: vec![],
            sort: None,
            limit: None,
            offset: None,
            fields: None,
        };
        let err = p.query(&q).await.unwrap_err();
        assert!(matches!(err, Error::NotInitialized { .. }));
    }

    #[tokio::test]
    async fn init_requires_url() {
        let mut p = GiteaProvider::new();
        let cfg = ProviderConfig::default();
        let err = p.init(&cfg).await.unwrap_err();
        assert!(err.to_string().contains("url"));
    }

    #[tokio::test]
    async fn init_succeeds_with_url() {
        let mut p = GiteaProvider::new();
        let mut settings = std::collections::HashMap::new();
        settings.insert("url".into(), serde_json::json!("http://localhost:3000"));
        let cfg = ProviderConfig { settings };
        p.init(&cfg).await.unwrap();
        assert!(p.client.is_some());
    }

    #[tokio::test]
    async fn issues_query_requires_owner_repo() {
        let mut p = GiteaProvider::new();
        let mut settings = std::collections::HashMap::new();
        settings.insert("url".into(), serde_json::json!("http://localhost:3000"));
        let cfg = ProviderConfig { settings };
        p.init(&cfg).await.unwrap();

        let q = Query {
            capability: Capability::Issues,
            filters: vec![],
            sort: None,
            limit: None,
            offset: None,
            fields: None,
        };
        let err = p.query(&q).await.unwrap_err();
        assert!(err.to_string().contains("owner"));
    }

    #[test]
    fn extract_owner_repo_from_filters() {
        let filters = vec![
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
        ];
        let (owner, repo) = GiteaProvider::extract_owner_repo(&filters);
        assert_eq!(owner, Some("gastownhall"));
        assert_eq!(repo, Some("gascity"));
    }

    #[test]
    fn page_params_defaults() {
        let q = Query {
            capability: Capability::Repos,
            filters: vec![],
            sort: None,
            limit: None,
            offset: None,
            fields: None,
        };
        let (page, limit) = GiteaProvider::page_params(&q);
        assert_eq!(page, 1);
        assert_eq!(limit, 30);
    }

    #[test]
    fn page_params_with_offset() {
        let q = Query {
            capability: Capability::Repos,
            filters: vec![],
            sort: None,
            limit: Some(10),
            offset: Some(20),
            fields: None,
        };
        let (page, limit) = GiteaProvider::page_params(&q);
        assert_eq!(page, 3);
        assert_eq!(limit, 10);
    }

    #[test]
    fn page_params_caps_limit_at_50() {
        let q = Query {
            capability: Capability::Repos,
            filters: vec![],
            sort: None,
            limit: Some(100),
            offset: None,
            fields: None,
        };
        let (_, limit) = GiteaProvider::page_params(&q);
        assert_eq!(limit, 50);
    }

    #[test]
    fn repos_to_result_empty() {
        let result = repos_to_result(vec![]);
        assert_eq!(result.rows.len(), 0);
        assert_eq!(result.total, Some(0));
        assert_eq!(result.columns.len(), 14);
    }

    #[test]
    fn users_to_result_maps_fields() {
        let users = vec![GiteaUser {
            id: 1,
            login: "alice".into(),
            full_name: Some("Alice A".into()),
            email: Some("alice@example.com".into()),
            avatar_url: None,
            created: None,
        }];
        let result = users_to_result(users);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][1], Value::Str("alice".into()));
        assert_eq!(result.rows[0][2], Value::Str("Alice A".into()));
    }

    #[test]
    fn project_fields_filters_columns() {
        let result = QueryResult {
            columns: vec![
                Column { name: "id".into(), col_type: ColumnType::Int },
                Column { name: "name".into(), col_type: ColumnType::Str },
                Column { name: "extra".into(), col_type: ColumnType::Str },
            ],
            rows: vec![vec![
                Value::Int(1),
                Value::Str("test".into()),
                Value::Str("x".into()),
            ]],
            total: Some(1),
            provider: "test".into(),
            latency_ms: 0,
        };

        let projected = project_fields(result, &["id".into(), "name".into()]);
        assert_eq!(projected.columns.len(), 2);
        assert_eq!(projected.rows[0].len(), 2);
        assert_eq!(projected.columns[0].name, "id");
        assert_eq!(projected.columns[1].name, "name");
    }

    #[test]
    fn filter_str_extracts_value() {
        let filters = vec![Filter {
            field: "state".into(),
            op: FilterOp::Eq,
            value: Value::Str("open".into()),
        }];
        assert_eq!(GiteaProvider::filter_str(&filters, "state"), Some("open"));
        assert_eq!(GiteaProvider::filter_str(&filters, "missing"), None);
    }

    #[test]
    fn default_trait_works() {
        let p = GiteaProvider::default();
        assert_eq!(p.name(), "gitea");
    }
}
