use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::Capability;

/// A query against a specific capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub capability: Capability,
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub sort: Option<Sort>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub fields: Option<Vec<String>>,
}

/// A filter condition on a field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub op: FilterOp,
    pub value: Value,
}

/// Filter comparison operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Gte,
    Lte,
    Contains,
    In,
    StartsWith,
}

/// Sort specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sort {
    pub field: String,
    pub dir: SortDir,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDir {
    Asc,
    Desc,
}

/// A column descriptor in query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub col_type: ColumnType,
}

/// Column data types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Bool,
    Int,
    Float,
    Str,
    Timestamp,
    List,
    Map,
}

/// The result of a query, returned as columnar data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
    pub total: Option<usize>,
    pub provider: String,
    pub latency_ms: u64,
}

/// A dynamically-typed value in query results and filters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Timestamp(DateTime<Utc>),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_serializes_roundtrip() {
        let q = Query {
            capability: Capability::Beads,
            filters: vec![Filter {
                field: "status".into(),
                op: FilterOp::Eq,
                value: Value::Str("open".into()),
            }],
            sort: Some(Sort {
                field: "created".into(),
                dir: SortDir::Desc,
            }),
            limit: Some(50),
            offset: Some(0),
            fields: Some(vec!["id".into(), "title".into()]),
        };
        let json = serde_json::to_string(&q).unwrap();
        let back: Query = serde_json::from_str(&json).unwrap();
        assert_eq!(back.capability, Capability::Beads);
        assert_eq!(back.filters.len(), 1);
        assert_eq!(back.limit, Some(50));
    }

    #[test]
    fn value_null_serializes() {
        let v = Value::Null;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn value_types_roundtrip() {
        let values = vec![
            Value::Bool(true),
            Value::Int(42),
            Value::Float(3.14),
            Value::Str("hello".into()),
            Value::List(vec![Value::Int(1), Value::Int(2)]),
        ];
        for v in values {
            let json = serde_json::to_string(&v).unwrap();
            let back: Value = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn query_result_with_rows() {
        let result = QueryResult {
            columns: vec![
                Column {
                    name: "id".into(),
                    col_type: ColumnType::Str,
                },
                Column {
                    name: "title".into(),
                    col_type: ColumnType::Str,
                },
            ],
            rows: vec![vec![
                Value::Str("gc-123".into()),
                Value::Str("Fix bug".into()),
            ]],
            total: Some(1),
            provider: "test".into(),
            latency_ms: 5,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: QueryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back.columns.len(), 2);
        assert_eq!(back.rows.len(), 1);
        assert_eq!(back.provider, "test");
    }

    #[test]
    fn filter_op_serializes_snake_case() {
        let json = serde_json::to_string(&FilterOp::StartsWith).unwrap();
        assert_eq!(json, r#""starts_with""#);
    }

    #[test]
    fn empty_query_defaults() {
        let json = r#"{"capability":"beads"}"#;
        let q: Query = serde_json::from_str(json).unwrap();
        assert_eq!(q.capability, Capability::Beads);
        assert!(q.filters.is_empty());
        assert!(q.sort.is_none());
        assert!(q.limit.is_none());
    }
}
