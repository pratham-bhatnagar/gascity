use std::time::Instant;

use gt_monitor_core::{
    Capability, Column, ColumnType, Error, Filter, FilterOp, Provider, ProviderConfig,
    ProviderHealth, ProviderStatus, Query, QueryResult, Result, Value,
};
use sysinfo::{Disks, System};

/// Provider for host-level system metrics: CPU, memory, disk, and processes.
///
/// Reads live data from the OS via the `sysinfo` crate. No external services
/// required — works anywhere the binary can read /proc (Linux) or equivalent.
pub struct SystemProvider {
    name: String,
}

impl SystemProvider {
    pub fn new() -> Self {
        SystemProvider {
            name: "system".to_string(),
        }
    }

    fn query_cpu(&self, q: &Query) -> Result<QueryResult> {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        // sysinfo needs a short delay between refreshes for accurate per-core usage
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_all();

        let columns = vec![
            Column { name: "core".into(), col_type: ColumnType::Int },
            Column { name: "usage_percent".into(), col_type: ColumnType::Float },
            Column { name: "frequency_mhz".into(), col_type: ColumnType::Int },
            Column { name: "vendor_id".into(), col_type: ColumnType::Str },
            Column { name: "brand".into(), col_type: ColumnType::Str },
        ];

        let mut rows: Vec<Vec<Value>> = sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| {
                vec![
                    Value::Int(i as i64),
                    Value::Float(cpu.cpu_usage() as f64),
                    Value::Int(cpu.frequency() as i64),
                    Value::Str(cpu.vendor_id().to_string()),
                    Value::Str(cpu.brand().to_string()),
                ]
            })
            .collect();

        apply_filters(&mut rows, &q.filters, &columns);
        let total = rows.len();
        apply_pagination(&mut rows, q.limit, q.offset);
        let columns = project_columns(columns, &rows, &q.fields);

        Ok(QueryResult {
            columns: columns.0,
            rows: columns.1,
            total: Some(total),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn query_memory(&self, q: &Query) -> Result<QueryResult> {
        let mut sys = System::new();
        sys.refresh_memory();

        let columns = vec![
            Column { name: "total_bytes".into(), col_type: ColumnType::Int },
            Column { name: "used_bytes".into(), col_type: ColumnType::Int },
            Column { name: "free_bytes".into(), col_type: ColumnType::Int },
            Column { name: "available_bytes".into(), col_type: ColumnType::Int },
            Column { name: "swap_total_bytes".into(), col_type: ColumnType::Int },
            Column { name: "swap_used_bytes".into(), col_type: ColumnType::Int },
        ];

        let rows = vec![vec![
            Value::Int(sys.total_memory() as i64),
            Value::Int(sys.used_memory() as i64),
            Value::Int(sys.free_memory() as i64),
            Value::Int(sys.available_memory() as i64),
            Value::Int(sys.total_swap() as i64),
            Value::Int(sys.used_swap() as i64),
        ]];

        let (columns, rows) = project_columns(columns, &rows, &q.fields);

        Ok(QueryResult {
            columns,
            rows,
            total: Some(1),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn query_disk(&self, q: &Query) -> Result<QueryResult> {
        let disks = Disks::new_with_refreshed_list();

        let columns = vec![
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "mount_point".into(), col_type: ColumnType::Str },
            Column { name: "file_system".into(), col_type: ColumnType::Str },
            Column { name: "total_bytes".into(), col_type: ColumnType::Int },
            Column { name: "available_bytes".into(), col_type: ColumnType::Int },
            Column { name: "is_removable".into(), col_type: ColumnType::Bool },
        ];

        let mut rows: Vec<Vec<Value>> = disks
            .iter()
            .map(|d| {
                vec![
                    Value::Str(d.name().to_string_lossy().to_string()),
                    Value::Str(d.mount_point().to_string_lossy().to_string()),
                    Value::Str(d.file_system().to_string_lossy().to_string()),
                    Value::Int(d.total_space() as i64),
                    Value::Int(d.available_space() as i64),
                    Value::Bool(d.is_removable()),
                ]
            })
            .collect();

        apply_filters(&mut rows, &q.filters, &columns);
        let total = rows.len();
        apply_pagination(&mut rows, q.limit, q.offset);
        let columns = project_columns(columns, &rows, &q.fields);

        Ok(QueryResult {
            columns: columns.0,
            rows: columns.1,
            total: Some(total),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }

    fn query_processes(&self, q: &Query) -> Result<QueryResult> {
        let mut sys = System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let columns = vec![
            Column { name: "pid".into(), col_type: ColumnType::Int },
            Column { name: "name".into(), col_type: ColumnType::Str },
            Column { name: "cpu_usage".into(), col_type: ColumnType::Float },
            Column { name: "memory_bytes".into(), col_type: ColumnType::Int },
            Column { name: "status".into(), col_type: ColumnType::Str },
            Column { name: "cmd".into(), col_type: ColumnType::Str },
        ];

        let mut rows: Vec<Vec<Value>> = sys
            .processes()
            .iter()
            .map(|(pid, proc_)| {
                vec![
                    Value::Int(pid.as_u32() as i64),
                    Value::Str(proc_.name().to_string_lossy().to_string()),
                    Value::Float(proc_.cpu_usage() as f64),
                    Value::Int(proc_.memory() as i64),
                    Value::Str(format!("{:?}", proc_.status())),
                    Value::Str(
                        proc_
                            .cmd()
                            .iter()
                            .map(|s| s.to_string_lossy().to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                ]
            })
            .collect();

        apply_filters(&mut rows, &q.filters, &columns);
        let total = rows.len();
        apply_pagination(&mut rows, q.limit, q.offset);
        let columns = project_columns(columns, &rows, &q.fields);

        Ok(QueryResult {
            columns: columns.0,
            rows: columns.1,
            total: Some(total),
            provider: self.name.clone(),
            latency_ms: 0,
        })
    }
}

impl Default for SystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Provider for SystemProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::SystemCpu,
            Capability::SystemMemory,
            Capability::SystemDisk,
            Capability::SystemProcesses,
        ]
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth {
            name: self.name.clone(),
            status: ProviderStatus::Healthy,
            latency_ms: Some(0),
            message: None,
        }
    }

    async fn query(&self, q: &Query) -> Result<QueryResult> {
        let start = Instant::now();
        let mut result = match q.capability {
            Capability::SystemCpu => self.query_cpu(q)?,
            Capability::SystemMemory => self.query_memory(q)?,
            Capability::SystemDisk => self.query_disk(q)?,
            Capability::SystemProcesses => self.query_processes(q)?,
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

    async fn init(&mut self, _config: &ProviderConfig) -> Result<()> {
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
        FilterOp::Gt => cmp_values(val, target).map_or(false, |o| o == std::cmp::Ordering::Greater),
        FilterOp::Lt => cmp_values(val, target).map_or(false, |o| o == std::cmp::Ordering::Less),
        FilterOp::Gte => cmp_values(val, target).map_or(false, |o| o != std::cmp::Ordering::Less),
        FilterOp::Lte => {
            cmp_values(val, target).map_or(false, |o| o != std::cmp::Ordering::Greater)
        }
        FilterOp::Contains => match (val, target) {
            (Value::Str(s), Value::Str(t)) => s.contains(t.as_str()),
            _ => false,
        },
        FilterOp::StartsWith => match (val, target) {
            (Value::Str(s), Value::Str(t)) => s.starts_with(t.as_str()),
            _ => false,
        },
        FilterOp::In => match target {
            Value::List(list) => list.contains(val),
            _ => false,
        },
    }
}

fn cmp_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
        (Value::Str(a), Value::Str(b)) => Some(a.cmp(b)),
        _ => None,
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
        let p = SystemProvider::new();
        assert_eq!(p.name(), "system");
    }

    #[test]
    fn provider_capabilities() {
        let p = SystemProvider::new();
        let caps = p.capabilities();
        assert!(caps.contains(&Capability::SystemCpu));
        assert!(caps.contains(&Capability::SystemMemory));
        assert!(caps.contains(&Capability::SystemDisk));
        assert!(caps.contains(&Capability::SystemProcesses));
        assert_eq!(caps.len(), 4);
    }

    #[tokio::test]
    async fn health_is_healthy() {
        let p = SystemProvider::new();
        let h = p.health().await;
        assert_eq!(h.status, ProviderStatus::Healthy);
        assert_eq!(h.name, "system");
    }

    #[tokio::test]
    async fn init_succeeds() {
        let mut p = SystemProvider::new();
        let cfg = ProviderConfig::default();
        assert!(p.init(&cfg).await.is_ok());
    }

    #[tokio::test]
    async fn query_cpu_returns_columns() {
        let p = SystemProvider::new();
        let result = p.query(&make_query(Capability::SystemCpu)).await.unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"core"));
        assert!(col_names.contains(&"usage_percent"));
        assert!(col_names.contains(&"frequency_mhz"));
        assert!(!result.rows.is_empty());
    }

    #[tokio::test]
    async fn query_memory_returns_single_row() {
        let p = SystemProvider::new();
        let result = p
            .query(&make_query(Capability::SystemMemory))
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"total_bytes"));
        assert!(col_names.contains(&"used_bytes"));
        assert!(col_names.contains(&"free_bytes"));
        // total_bytes should be positive
        if let Value::Int(total) = &result.rows[0][0] {
            assert!(*total > 0);
        } else {
            panic!("expected Int for total_bytes");
        }
    }

    #[tokio::test]
    async fn query_disk_returns_rows() {
        let p = SystemProvider::new();
        let result = p
            .query(&make_query(Capability::SystemDisk))
            .await
            .unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"mount_point"));
        assert!(col_names.contains(&"total_bytes"));
        // Should have at least one disk
        assert!(!result.rows.is_empty());
    }

    #[tokio::test]
    async fn query_processes_returns_rows() {
        let p = SystemProvider::new();
        let result = p
            .query(&make_query(Capability::SystemProcesses))
            .await
            .unwrap();
        let col_names: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(col_names.contains(&"pid"));
        assert!(col_names.contains(&"name"));
        assert!(col_names.contains(&"memory_bytes"));
        // Should have at least one process (our own)
        assert!(!result.rows.is_empty());
    }

    #[tokio::test]
    async fn query_unsupported_capability_errors() {
        let p = SystemProvider::new();
        let err = p.query(&make_query(Capability::Beads)).await.unwrap_err();
        match err {
            Error::ProviderError { provider, .. } => assert_eq!(provider, "system"),
            other => panic!("expected ProviderError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn query_with_limit() {
        let p = SystemProvider::new();
        let mut q = make_query(Capability::SystemProcesses);
        q.limit = Some(3);
        let result = p.query(&q).await.unwrap();
        assert!(result.rows.len() <= 3);
    }

    #[tokio::test]
    async fn query_with_offset() {
        let p = SystemProvider::new();
        // Use CPU cores which are stable (don't change between calls)
        let all = p
            .query(&make_query(Capability::SystemCpu))
            .await
            .unwrap();
        let mut q = make_query(Capability::SystemCpu);
        q.offset = Some(1);
        let offset_result = p.query(&q).await.unwrap();
        if all.rows.len() > 1 {
            assert_eq!(offset_result.rows.len(), all.rows.len() - 1);
        }
    }

    #[tokio::test]
    async fn query_with_field_projection() {
        let p = SystemProvider::new();
        let mut q = make_query(Capability::SystemMemory);
        q.fields = Some(vec!["total_bytes".into(), "used_bytes".into()]);
        let result = p.query(&q).await.unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "total_bytes");
        assert_eq!(result.columns[1].name, "used_bytes");
        assert_eq!(result.rows[0].len(), 2);
    }

    #[tokio::test]
    async fn query_cpu_with_filter() {
        let p = SystemProvider::new();
        let mut q = make_query(Capability::SystemCpu);
        q.filters = vec![Filter {
            field: "core".into(),
            op: FilterOp::Eq,
            value: Value::Int(0),
        }];
        let result = p.query(&q).await.unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0], Value::Int(0));
    }

    // --- helper tests ---

    #[test]
    fn value_matches_eq() {
        assert!(value_matches(
            &Value::Int(5),
            &FilterOp::Eq,
            &Value::Int(5)
        ));
        assert!(!value_matches(
            &Value::Int(5),
            &FilterOp::Eq,
            &Value::Int(6)
        ));
    }

    #[test]
    fn value_matches_contains() {
        assert!(value_matches(
            &Value::Str("hello world".into()),
            &FilterOp::Contains,
            &Value::Str("world".into()),
        ));
    }

    #[test]
    fn value_matches_in() {
        assert!(value_matches(
            &Value::Int(3),
            &FilterOp::In,
            &Value::List(vec![Value::Int(1), Value::Int(3), Value::Int(5)]),
        ));
    }

    #[test]
    fn pagination_offset_beyond_length() {
        let mut rows = vec![vec![Value::Int(1)], vec![Value::Int(2)]];
        apply_pagination(&mut rows, None, Some(10));
        assert!(rows.is_empty());
    }

    #[test]
    fn default_impl() {
        let p = SystemProvider::default();
        assert_eq!(p.name(), "system");
    }
}
