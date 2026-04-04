mod capability;
mod engine;
mod error;
mod provider;
mod query;

pub use capability::Capability;
pub use engine::{Engine, EngineConfig, SystemHealth};
pub use error::Error;
pub use provider::{Provider, ProviderConfig, ProviderHealth, ProviderStatus};
pub use query::{Column, ColumnType, Filter, FilterOp, Query, QueryResult, Sort, SortDir, Value};

pub type Result<T> = std::result::Result<T, Error>;
