use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use chrono::{DateTime, Utc};
use gt_monitor_core::{
    Capability, Column, ColumnType, Error, Filter, FilterOp, Provider, ProviderConfig,
    ProviderHealth, ProviderStatus, Query, QueryResult, Result, Value,
};

/// Provider for local git repository data: commits and branches.
///
/// Shells out to `git` CLI commands to read repository state. Requires `git`
/// to be on `$PATH` and a valid repo at the configured `repo_path`.
pub struct GitProvider {
    name: String,
    repo_path: Option<PathBuf>,
}

impl GitProvider {
    pub fn new() -> Self {
        GitProvider {
            name: "git".to_string(),
            repo_path: None,
        }
    }

    fn git_cmd(&self) -> Command {
        let mut cmd = Command::new("git");
        if let Some(ref path) = self.repo_path {
            cmd.arg("-C").arg(path);
        }
        cmd
    }

    fn query_commits(&self, q: &Query) -> Result<QueryResult> {
        let limit = q.limit.unwrap_or(100);
        let mut cmd = self.git_cmd();
        cmd.args([
            "log",
            &format!("-{limit}"),
            "--format=%H%x00%an%x00%ae%x00%aI%x00%s",
        ]);

        // Apply author filter if present
        for filter in &q.filters {
            if filter.field == "author" && filter.op == FilterOp::Eq {
                if let Value::Str(ref author) = filter.value {
                    cmd.arg(format!("--author={author}"));
                }
            }
            if filter.field == "since" && filter.op == FilterOp::Gte {
                if let Value::Str(ref since) = filter.value {
                    cmd.arg(format!("--since={since}"));
                }
            }
        }

        let output = cmd.output().map_err(|e| Error::ProviderError {
            provider: self.name.clone(),
            message: format!("failed to run git log: {e}"),
        })?;

        if !output.status.success() {
            return Err(Error::ProviderError {
                provider: self.name.clone(),
                message: format!(
                    "git log failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }

        let columns = vec![
            Column { name: "hash".into(), col_type: ColumnType::Str },
            Column { name: "author_name".into(), col_type: ColumnType::Str },
            Column { name: "author_email".into(), col_type: ColumnType::Str },
            Column { name: "date".into(), col_type: ColumnType::Timestamp },
            Column { name: "subject".into(), col_type: ColumnType::Str },
        ];

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut rows: Vec<Vec<Value>> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(5, '\0').collect();
                if parts.len() < 5 {
                    return None;
                }
                let date = parts[3]
                    .parse::<DateTime<Utc>>()
                    .map(Value::Timestamp)
                    .unwrap_or(Value::Str(parts[3].to_string()));
                Some(vec![
                    Value::Str(parts[0].to_string()),
                    Value::Str(parts[1].to_string()),
                    Value::Str(parts[2].to_string()),
                    date,
                    Value::Str(parts[4].to_string()),
                ])
            })
            .collect();

        // Apply non-git filters (e.g. subject contains)
        apply_filters(&mut rows, &q.filters, &columns);

        let total = rows.len();
        apply_pagination(&mut rows, q.limit, q.offset);
        let (columns, rows) = project_columns(columns, &rows, &q.fields);

        Ok(QueryResult {
            columns,
            rows,
            total: Some(total),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn query_branches(&self, q: &Query) -> Result<QueryResult> {
        let mut cmd = self.git_cmd();
        cmd.args(["branch", "-a", "--format=%(refname:short)%00%(objectname:short)%00%(committerdate:iso-strict)%00%(HEAD)"]);

        let output = cmd.output().map_err(|e| Error::ProviderError {
            provider: self.name.clone(),
            message: format!("failed to run git branch: {e}"),
        })?;

        if !output.status.success() {
            return Err(Error::ProviderError {
                provider: self.name.clone(),
                message: format!(
                    "git branch failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }

        let columns = vec![
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "head_sha".into(), col_type: ColumnType::Str },
            Column { name: "last_commit_date".into(), col_type: ColumnType::Str },
            Column { name: "is_current".into(), col_type: ColumnType::Bool },
        ];

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut rows: Vec<Vec<Value>> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(4, '\0').collect();
                if parts.len() < 4 {
                    return None;
                }
                Some(vec![
                    Value::Str(parts[0].to_string()),
                    Value::Str(parts[1].to_string()),
                    Value::Str(parts[2].to_string()),
                    Value::Bool(parts[3].trim() == "*"),
                ])
            })
            .collect();

        apply_filters(&mut rows, &q.filters, &columns);
        let total = rows.len();
        apply_pagination(&mut rows, q.limit, q.offset);
        let (columns, rows) = project_columns(columns, &rows, &q.fields);

        Ok(QueryResult {
            columns,
            rows,
            total: Some(total),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }
}

impl Default for GitProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Provider for GitProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Commits, Capability::Branches]
    }

    async fn health(&self) -> ProviderHealth {
        let mut cmd = self.git_cmd();
        cmd.args(["rev-parse", "--git-dir"]);
        match cmd.output() {
            Ok(output) if output.status.success() => ProviderHealth {
                name: self.name.clone(),
                status: ProviderStatus::Healthy,
                latency_ms: Some(0),
                message: None,
            },
            Ok(output) => ProviderHealth {
                name: self.name.clone(),
                status: ProviderStatus::Unavailable,
                latency_ms: Some(0),
                message: Some(String::from_utf8_lossy(&output.stderr).to_string()),
            },
            Err(e) => ProviderHealth {
                name: self.name.clone(),
                status: ProviderStatus::Unavailable,
                latency_ms: None,
                message: Some(format!("git not available: {e}")),
            },
        }
    }

    async fn query(&self, q: &Query) -> Result<QueryResult> {
        let start = Instant::now();
        let mut result = match q.capability {
            Capability::Commits => self.query_commits(q)?,
            Capability::Branches => self.query_branches(q)?,
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
        if let Some(path) = config.settings.get("repo_path").and_then(|v| v.as_str()) {
            self.repo_path = Some(PathBuf::from(path));
        }
        Ok(())
    }
}

// --- Shared filter/pagination/projection helpers ---

fn column_index(columns: &[Column], field: &str) -> Option<usize> {
    columns.iter().position(|c| c.name == field)
}

fn value_matches(val: &Value, op: &FilterOp, target: &Value) -> bool {
    match op {
        FilterOp::Eq => val == target,
        FilterOp::Ne => val != target,
        FilterOp::Contains => match (val, target) {
            (Value::Str(s), Value::Str(t)) => s.contains(t.as_str()),
            _ => false,
        },
        FilterOp::StartsWith => match (val, target) {
            (Value::Str(s), Value::Str(t)) => s.starts_with(t.as_str()),
            _ => false,
        },
        // author/since filters are already applied at git level
        _ => true,
    }
}

fn apply_filters(rows: &mut Vec<Vec<Value>>, filters: &[Filter], columns: &[Column]) {
    for filter in filters {
        // Skip filters already handled by git args
        if filter.field == "author" || filter.field == "since" {
            continue;
        }
        if let Some(idx) = column_index(columns, &filter.field) {
            rows.retain(|row| {
                row.get(idx)
                    .map_or(false, |v| value_matches(v, &filter.op, &filter.value))
            });
        }
    }
}

fn apply_pagination(rows: &mut Vec<Vec<Value>>, limit: Option<usize>, offset: Option<usize>) {
    if let Some(off) = offset {
        if off < rows.len() {
            *rows = rows.split_off(off);
        } else {
            rows.clear();
        }
    }
    if let Some(lim) = limit {
        rows.truncate(lim);
    }
}

fn project_columns(
    columns: Vec<Column>,
    rows: &[Vec<Value>],
    fields: &Option<Vec<String>>,
) -> (Vec<Column>, Vec<Vec<Value>>) {
    let Some(fields) = fields else {
        return (columns, rows.to_vec());
    };

    let indices: Vec<usize> = fields
        .iter()
        .filter_map(|f| column_index(&columns, f))
        .collect();

    let projected_columns: Vec<Column> = indices.iter().map(|&i| columns[i].clone()).collect();
    let projected_rows: Vec<Vec<Value>> = rows
        .iter()
        .map(|row| indices.iter().map(|&i| row[i].clone()).collect())
        .collect();

    (projected_columns, projected_rows)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let p = GitProvider::new();
        assert_eq!(p.name(), "git");
    }

    #[test]
    fn provider_capabilities() {
        let p = GitProvider::new();
        let caps = p.capabilities();
        assert!(caps.contains(&Capability::Commits));
        assert!(caps.contains(&Capability::Branches));
        assert_eq!(caps.len(), 2);
    }

    #[tokio::test]
    async fn health_in_git_repo() {
        // We're running inside a git repo, so health should be healthy
        let p = GitProvider::new();
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Healthy);
    }

    #[tokio::test]
    async fn health_outside_git_repo() {
        let mut p = GitProvider::new();
        p.repo_path = Some(PathBuf::from("/tmp"));
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Unavailable);
    }

    #[tokio::test]
    async fn init_sets_repo_path() {
        let mut p = GitProvider::new();
        let mut settings = std::collections::HashMap::new();
        settings.insert(
            "repo_path".to_string(),
            serde_json::Value::String("/tmp/test".to_string()),
        );
        let cfg = ProviderConfig { settings };
        p.init(&cfg).await.unwrap();
        assert_eq!(p.repo_path, Some(PathBuf::from("/tmp/test")));
    }

    #[tokio::test]
    async fn query_commits_returns_results() {
        let p = GitProvider::new();
        let mut q = make_query(Capability::Commits);
        q.limit = Some(5);
        let result = p.query(&q).await.unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"hash"));
        assert!(col_names.contains(&"author_name"));
        assert!(col_names.contains(&"subject"));
        assert!(!result.rows.is_empty());
    }

    #[tokio::test]
    async fn query_commits_with_subject_filter() {
        let p = GitProvider::new();
        let mut q = make_query(Capability::Commits);
        q.limit = Some(50);
        q.filters = vec![Filter {
            field: "subject".into(),
            op: FilterOp::Contains,
            value: Value::Str("fix".into()),
        }];
        let result = p.query(&q).await.unwrap();
        // All returned subjects should contain "fix"
        let subject_idx = result
            .columns
            .iter()
            .position(|c| c.name == "subject")
            .unwrap();
        for row in &result.rows {
            if let Value::Str(ref s) = row[subject_idx] {
                assert!(
                    s.contains("fix"),
                    "subject {s:?} doesn't contain 'fix'"
                );
            }
        }
    }

    #[tokio::test]
    async fn query_branches_returns_results() {
        let p = GitProvider::new();
        let result = p.query(&make_query(Capability::Branches)).await.unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"name"));
        assert!(col_names.contains(&"head_sha"));
        assert!(col_names.contains(&"is_current"));
        assert!(!result.rows.is_empty());
    }

    #[tokio::test]
    async fn query_commits_with_field_projection() {
        let p = GitProvider::new();
        let mut q = make_query(Capability::Commits);
        q.limit = Some(3);
        q.fields = Some(vec!["hash".into(), "subject".into()]);
        let result = p.query(&q).await.unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "hash");
        assert_eq!(result.columns[1].name, "subject");
    }

    #[tokio::test]
    async fn query_unsupported_capability_errors() {
        let p = GitProvider::new();
        let err = p.query(&make_query(Capability::Beads)).await.unwrap_err();
        match err {
            Error::ProviderError { provider, .. } => assert_eq!(provider, "git"),
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[test]
    fn default_impl() {
        let p = GitProvider::default();
        assert_eq!(p.name(), "git");
        assert!(p.repo_path.is_none());
    }
}
