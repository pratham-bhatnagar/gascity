use std::path::{Path, PathBuf};
use std::time::Instant;

use gt_monitor_core::{
    Capability, Column, ColumnType, Error, Filter, FilterOp, Provider, ProviderConfig,
    ProviderHealth, ProviderStatus, Query, QueryResult, Result, Value,
};

/// Provider for Gas Town configuration data: agent configs, rig configs,
/// town config, formulas, crontabs, and knowledge base entries.
///
/// Reads TOML/JSON configuration files from the Gas Town directory structure.
///
/// Configuration:
/// - `town_path`: Path to the Gas Town root directory (e.g. "/home/user/gt")
pub struct ConfigProvider {
    name: String,
    town_path: Option<PathBuf>,
}

impl ConfigProvider {
    pub fn new() -> Self {
        ConfigProvider {
            name: "config".to_string(),
            town_path: None,
        }
    }

    fn town_path(&self) -> Result<&Path> {
        self.town_path.as_deref().ok_or_else(|| Error::ProviderError {
            provider: self.name.clone(),
            message: "town_path not configured".to_string(),
        })
    }

    fn query_agent_configs(&self, q: &Query) -> Result<QueryResult> {
        let town = self.town_path()?;
        let city_toml = town.join("city.toml");

        let columns = vec![
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "role".into(), col_type: ColumnType::Str },
            Column { name: "model".into(), col_type: ColumnType::Str },
            Column { name: "rig".into(), col_type: ColumnType::Str },
            Column { name: "pool_size".into(), col_type: ColumnType::Int },
            Column { name: "raw_toml".into(), col_type: ColumnType::Str },
        ];

        let mut rows = Vec::new();

        if city_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&city_toml) {
                if let Ok(table) = content.parse::<toml::Table>() {
                    if let Some(toml::Value::Array(agents)) = table.get("agent") {
                        for agent in agents {
                            if let toml::Value::Table(ref t) = agent {
                                let name = t
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let role = t
                                    .get("role")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let model = t
                                    .get("model")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let rig = t
                                    .get("rig")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let pool_size = t
                                    .get("pool_size")
                                    .and_then(|v| v.as_integer())
                                    .unwrap_or(1);
                                let raw = toml::to_string_pretty(t).unwrap_or_default();

                                rows.push(vec![
                                    Value::Str(name),
                                    Value::Str(role),
                                    Value::Str(model),
                                    Value::Str(rig),
                                    Value::Int(pool_size),
                                    Value::Str(raw),
                                ]);
                            }
                        }
                    }
                }
            }
        }

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

    fn query_rig_configs(&self, q: &Query) -> Result<QueryResult> {
        let town = self.town_path()?;

        let columns = vec![
            Column { name: "rig_name".into(), col_type: ColumnType::Str },
            Column { name: "path".into(), col_type: ColumnType::Str },
            Column { name: "has_claude_md".into(), col_type: ColumnType::Bool },
            Column { name: "has_agents_md".into(), col_type: ColumnType::Bool },
        ];

        let mut rows = Vec::new();

        // Scan for rig directories (directories containing CLAUDE.md or a known structure)
        if let Ok(entries) = std::fs::read_dir(town) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Skip hidden dirs and known non-rig dirs
                    if name.starts_with('.') || name == "docs" || name == "mayor" {
                        continue;
                    }
                    let has_claude = path.join("CLAUDE.md").exists();
                    let has_agents = path.join("AGENTS.md").exists();
                    if has_claude || has_agents {
                        rows.push(vec![
                            Value::Str(name),
                            Value::Str(path.to_string_lossy().to_string()),
                            Value::Bool(has_claude),
                            Value::Bool(has_agents),
                        ]);
                    }
                }
            }
        }

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

    fn query_town_config(&self, _q: &Query) -> Result<QueryResult> {
        let town = self.town_path()?;
        let city_toml = town.join("city.toml");

        let columns = vec![
            Column { name: "key".into(), col_type: ColumnType::Str },
            Column { name: "value".into(), col_type: ColumnType::Str },
            Column { name: "section".into(), col_type: ColumnType::Str },
        ];

        let mut rows = Vec::new();

        if city_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&city_toml) {
                if let Ok(table) = content.parse::<toml::Table>() {
                    flatten_toml("", &toml::Value::Table(table), &mut rows);
                }
            }
        }

        Ok(QueryResult {
            columns,
            rows,
            total: None,
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn query_formulas(&self, q: &Query) -> Result<QueryResult> {
        let town = self.town_path()?;
        let formulas_dir = town.join(".beads").join("formulas");

        let columns = vec![
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "path".into(), col_type: ColumnType::Str },
            Column { name: "size_bytes".into(), col_type: ColumnType::Int },
        ];

        let mut rows = Vec::new();

        if formulas_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&formulas_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "toml") {
                        let name = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let size = entry.metadata().map(|m| m.len() as i64).unwrap_or(0);
                        rows.push(vec![
                            Value::Str(name),
                            Value::Str(path.to_string_lossy().to_string()),
                            Value::Int(size),
                        ]);
                    }
                }
            }
        }

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

    fn query_crontabs(&self, q: &Query) -> Result<QueryResult> {
        let town = self.town_path()?;
        let city_toml = town.join("city.toml");

        let columns = vec![
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "schedule".into(), col_type: ColumnType::Str },
            Column { name: "command".into(), col_type: ColumnType::Str },
            Column { name: "enabled".into(), col_type: ColumnType::Bool },
        ];

        let mut rows = Vec::new();

        if city_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&city_toml) {
                if let Ok(table) = content.parse::<toml::Table>() {
                    if let Some(toml::Value::Array(crons)) = table.get("cron") {
                        for cron in crons {
                            if let toml::Value::Table(ref t) = cron {
                                let name = t
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let schedule = t
                                    .get("schedule")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let command = t
                                    .get("command")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let enabled = t
                                    .get("enabled")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(true);
                                rows.push(vec![
                                    Value::Str(name),
                                    Value::Str(schedule),
                                    Value::Str(command),
                                    Value::Bool(enabled),
                                ]);
                            }
                        }
                    }
                }
            }
        }

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

    fn query_knowledge_base(&self, q: &Query) -> Result<QueryResult> {
        let town = self.town_path()?;
        let kb_dir = town.join("docs");

        let columns = vec![
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "path".into(), col_type: ColumnType::Str },
            Column { name: "size_bytes".into(), col_type: ColumnType::Int },
            Column { name: "extension".into(), col_type: ColumnType::Str },
        ];

        let mut rows = Vec::new();

        if kb_dir.exists() {
            collect_docs(&kb_dir, &mut rows);
        }

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

fn collect_docs(dir: &Path, rows: &mut Vec<Vec<Value>>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_docs(&path, rows);
            } else {
                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default();
                if matches!(ext.as_str(), "md" | "toml" | "json" | "yaml" | "yml" | "txt") {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let size = entry.metadata().map(|m| m.len() as i64).unwrap_or(0);
                    rows.push(vec![
                        Value::Str(name),
                        Value::Str(path.to_string_lossy().to_string()),
                        Value::Int(size),
                        Value::Str(ext),
                    ]);
                }
            }
        }
    }
}

fn flatten_toml(prefix: &str, value: &toml::Value, rows: &mut Vec<Vec<Value>>) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                match val {
                    toml::Value::Table(_) => {
                        flatten_toml(&full_key, val, rows);
                    }
                    toml::Value::Array(arr) => {
                        // For arrays of tables (e.g. [[agent]]), store count
                        let section = if prefix.is_empty() {
                            key.clone()
                        } else {
                            prefix.to_string()
                        };
                        rows.push(vec![
                            Value::Str(full_key),
                            Value::Str(format!("[{} items]", arr.len())),
                            Value::Str(section),
                        ]);
                    }
                    _ => {
                        let section = if prefix.is_empty() {
                            "root".to_string()
                        } else {
                            prefix.to_string()
                        };
                        rows.push(vec![
                            Value::Str(full_key),
                            Value::Str(format!("{val}")),
                            Value::Str(section),
                        ]);
                    }
                }
            }
        }
        _ => {
            rows.push(vec![
                Value::Str(prefix.to_string()),
                Value::Str(format!("{value}")),
                Value::Str("root".to_string()),
            ]);
        }
    }
}

impl Default for ConfigProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Provider for ConfigProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::AgentConfigs,
            Capability::RigConfigs,
            Capability::TownConfig,
            Capability::Formulas,
            Capability::Crontabs,
            Capability::KnowledgeBase,
        ]
    }

    async fn health(&self) -> ProviderHealth {
        match &self.town_path {
            Some(path) if path.exists() => ProviderHealth {
                name: self.name.clone(),
                status: ProviderStatus::Healthy,
                latency_ms: Some(0),
                message: None,
            },
            Some(path) => ProviderHealth {
                name: self.name.clone(),
                status: ProviderStatus::Unavailable,
                latency_ms: Some(0),
                message: Some(format!("town_path does not exist: {}", path.display())),
            },
            None => ProviderHealth {
                name: self.name.clone(),
                status: ProviderStatus::Degraded,
                latency_ms: Some(0),
                message: Some("town_path not configured".to_string()),
            },
        }
    }

    async fn query(&self, q: &Query) -> Result<QueryResult> {
        let start = Instant::now();
        let mut result = match q.capability {
            Capability::AgentConfigs => self.query_agent_configs(q)?,
            Capability::RigConfigs => self.query_rig_configs(q)?,
            Capability::TownConfig => self.query_town_config(q)?,
            Capability::Formulas => self.query_formulas(q)?,
            Capability::Crontabs => self.query_crontabs(q)?,
            Capability::KnowledgeBase => self.query_knowledge_base(q)?,
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
        if let Some(path) = config.settings.get("town_path").and_then(|v| v.as_str()) {
            self.town_path = Some(PathBuf::from(path));
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
        _ => true,
    }
}

fn apply_filters(rows: &mut Vec<Vec<Value>>, filters: &[Filter], columns: &[Column]) {
    for filter in filters {
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
    use std::collections::HashMap;
    use std::fs;

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

    /// Create a temporary Gas Town directory with a city.toml and typical structure.
    fn setup_town(dir: &Path) {
        let city_toml = r#"
[town]
name = "test-town"
version = "0.1.0"

[[agent]]
name = "mayor"
role = "coordinator"
model = "claude-sonnet-4-20250514"
rig = "gastown"
pool_size = 1

[[agent]]
name = "worker"
role = "polecat"
model = "claude-sonnet-4-20250514"
rig = "gastown"
pool_size = 3

[[cron]]
name = "health-check"
schedule = "*/5 * * * *"
command = "gt health"
enabled = true

[[cron]]
name = "backup"
schedule = "0 2 * * *"
command = "gt backup"
enabled = false
"#;
        fs::write(dir.join("city.toml"), city_toml).unwrap();

        // Create a rig directory
        let rig_dir = dir.join("gastown");
        fs::create_dir_all(&rig_dir).unwrap();
        fs::write(rig_dir.join("CLAUDE.md"), "# Rig Config").unwrap();
        fs::write(rig_dir.join("AGENTS.md"), "# Agents").unwrap();

        // Create formulas directory
        let formulas_dir = dir.join(".beads").join("formulas");
        fs::create_dir_all(&formulas_dir).unwrap();
        fs::write(
            formulas_dir.join("mol-polecat-work.toml"),
            "[formula]\nname = \"mol-polecat-work\"",
        )
        .unwrap();

        // Create docs directory
        let docs_dir = dir.join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("README.md"), "# Docs").unwrap();
        fs::write(docs_dir.join("guide.md"), "# Guide").unwrap();
    }

    #[test]
    fn provider_name() {
        let p = ConfigProvider::new();
        assert_eq!(p.name(), "config");
    }

    #[test]
    fn provider_capabilities() {
        let p = ConfigProvider::new();
        let caps = p.capabilities();
        assert!(caps.contains(&Capability::AgentConfigs));
        assert!(caps.contains(&Capability::RigConfigs));
        assert!(caps.contains(&Capability::TownConfig));
        assert!(caps.contains(&Capability::Formulas));
        assert!(caps.contains(&Capability::Crontabs));
        assert!(caps.contains(&Capability::KnowledgeBase));
        assert_eq!(caps.len(), 6);
    }

    #[tokio::test]
    async fn health_degraded_without_config() {
        let p = ConfigProvider::new();
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Degraded);
    }

    #[tokio::test]
    async fn health_unavailable_with_bad_path() {
        let mut p = ConfigProvider::new();
        p.town_path = Some(PathBuf::from("/nonexistent/path"));
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Unavailable);
    }

    #[tokio::test]
    async fn health_healthy_with_valid_path() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Healthy);
    }

    #[tokio::test]
    async fn init_sets_town_path() {
        let mut p = ConfigProvider::new();
        let mut settings = HashMap::new();
        settings.insert(
            "town_path".to_string(),
            serde_json::Value::String("/tmp/test".to_string()),
        );
        let cfg = ProviderConfig { settings };
        p.init(&cfg).await.unwrap();
        assert_eq!(p.town_path, Some(PathBuf::from("/tmp/test")));
    }

    #[tokio::test]
    async fn query_agent_configs() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let result = p
            .query(&make_query(Capability::AgentConfigs))
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 2); // mayor + worker
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"name"));
        assert!(col_names.contains(&"role"));
        assert!(col_names.contains(&"model"));

        // Check first agent is mayor
        assert_eq!(result.rows[0][0], Value::Str("mayor".to_string()));
        assert_eq!(result.rows[0][1], Value::Str("coordinator".to_string()));
    }

    #[tokio::test]
    async fn query_agent_configs_with_filter() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let mut q = make_query(Capability::AgentConfigs);
        q.filters = vec![Filter {
            field: "role".into(),
            op: FilterOp::Eq,
            value: Value::Str("polecat".into()),
        }];
        let result = p.query(&q).await.unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0], Value::Str("worker".to_string()));
    }

    #[tokio::test]
    async fn query_rig_configs() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let result = p
            .query(&make_query(Capability::RigConfigs))
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1); // gastown rig
        assert_eq!(result.rows[0][0], Value::Str("gastown".to_string()));
        assert_eq!(result.rows[0][2], Value::Bool(true)); // has_claude_md
        assert_eq!(result.rows[0][3], Value::Bool(true)); // has_agents_md
    }

    #[tokio::test]
    async fn query_town_config() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let result = p
            .query(&make_query(Capability::TownConfig))
            .await
            .unwrap();
        // Should have flattened keys from city.toml
        assert!(!result.rows.is_empty());
        let keys: Vec<&Value> = result.rows.iter().map(|r| &r[0]).collect();
        assert!(keys.contains(&&Value::Str("town.name".to_string())));
        assert!(keys.contains(&&Value::Str("town.version".to_string())));
    }

    #[tokio::test]
    async fn query_formulas() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let result = p
            .query(&make_query(Capability::Formulas))
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(
            result.rows[0][0],
            Value::Str("mol-polecat-work".to_string())
        );
    }

    #[tokio::test]
    async fn query_crontabs() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let result = p
            .query(&make_query(Capability::Crontabs))
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 2);
        assert_eq!(
            result.rows[0][0],
            Value::Str("health-check".to_string())
        );
        assert_eq!(result.rows[0][3], Value::Bool(true)); // enabled
        assert_eq!(result.rows[1][3], Value::Bool(false)); // disabled
    }

    #[tokio::test]
    async fn query_knowledge_base() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let result = p
            .query(&make_query(Capability::KnowledgeBase))
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 2); // README.md + guide.md
        let names: Vec<&Value> = result.rows.iter().map(|r| &r[0]).collect();
        assert!(names.contains(&&Value::Str("README.md".to_string())));
        assert!(names.contains(&&Value::Str("guide.md".to_string())));
    }

    #[tokio::test]
    async fn query_without_town_path_errors() {
        let p = ConfigProvider::new();
        let err = p
            .query(&make_query(Capability::AgentConfigs))
            .await
            .unwrap_err();
        match err {
            Error::ProviderError { message, .. } => {
                assert!(message.contains("town_path not configured"))
            }
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn query_unsupported_capability_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());
        let err = p.query(&make_query(Capability::Beads)).await.unwrap_err();
        match err {
            Error::ProviderError { provider, .. } => assert_eq!(provider, "config"),
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn query_with_field_projection() {
        let dir = tempfile::tempdir().unwrap();
        setup_town(dir.path());
        let mut p = ConfigProvider::new();
        p.town_path = Some(dir.path().to_path_buf());

        let mut q = make_query(Capability::AgentConfigs);
        q.fields = Some(vec!["name".into(), "role".into()]);
        let result = p.query(&q).await.unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "name");
        assert_eq!(result.columns[1].name, "role");
    }

    #[test]
    fn default_impl() {
        let p = ConfigProvider::default();
        assert_eq!(p.name(), "config");
        assert!(p.town_path.is_none());
    }

    #[test]
    fn flatten_toml_nested() {
        let toml_str = r#"
[server]
host = "localhost"
port = 8080

[server.tls]
enabled = true
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let mut rows = Vec::new();
        flatten_toml("", &toml::Value::Table(table), &mut rows);
        let keys: Vec<String> = rows
            .iter()
            .map(|r| {
                if let Value::Str(ref s) = r[0] {
                    s.clone()
                } else {
                    String::new()
                }
            })
            .collect();
        assert!(keys.contains(&"server.host".to_string()));
        assert!(keys.contains(&"server.port".to_string()));
        assert!(keys.contains(&"server.tls.enabled".to_string()));
    }
}
