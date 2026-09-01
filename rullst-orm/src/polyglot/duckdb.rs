use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use duckdb::{Connection, params_from_iter, types::ValueRef};

use super::{Backend, BackendCapabilities, Capability, PolyglotError};

const MAX_QUERY_ROWS: u32 = 10_000;
const MAX_SQL_BYTES: usize = 1024 * 1024;
const MAX_PARAMETERS: usize = 1_024;
const MAX_PARAMETER_BYTES: usize = 8 * 1024 * 1024;

/// Time units used by portable DuckDB temporal values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AnalyticsTimeUnit {
    /// Seconds.
    Second,
    /// Milliseconds.
    Millisecond,
    /// Microseconds.
    Microsecond,
    /// Nanoseconds.
    Nanosecond,
}

/// Scalar values accepted as parameters and returned by the analytics API.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AnalyticsValue {
    /// SQL `NULL`.
    Null,
    /// Boolean value.
    Boolean(bool),
    /// Signed integer up to 128 bits.
    Signed(i128),
    /// Unsigned integer up to 128 bits.
    Unsigned(u128),
    /// Double-precision floating point value.
    Float(f64),
    /// UTF-8 text.
    Text(String),
    /// Binary data.
    Bytes(Vec<u8>),
    /// DuckDB decimal preserving precision, scale, and scaled payload.
    Decimal {
        /// Declared decimal precision.
        precision: u8,
        /// Number of fractional digits.
        scale: u8,
        /// Scaled integer payload.
        scaled: i128,
    },
    /// DuckDB timestamp.
    Timestamp {
        /// Stored time unit.
        unit: AnalyticsTimeUnit,
        /// Value in the declared unit.
        value: i64,
    },
    /// Days since the Unix epoch.
    DateDays(i32),
    /// Time of day in the declared unit.
    Time {
        /// Stored time unit.
        unit: AnalyticsTimeUnit,
        /// Value in the declared unit.
        value: i64,
    },
    /// DuckDB interval components.
    Interval {
        /// Month component.
        months: i32,
        /// Day component.
        days: i32,
        /// Nanosecond component.
        nanos: i64,
    },
    /// Well-known binary geometry payload.
    Geometry(Vec<u8>),
}

/// One analytics result row keyed by column name.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyticsRow {
    columns: BTreeMap<String, AnalyticsValue>,
}

impl AnalyticsRow {
    /// Returns a value by its result column name.
    pub fn get(&self, name: &str) -> Option<&AnalyticsValue> {
        self.columns.get(name)
    }

    /// Consumes the row and returns all columns.
    pub fn into_columns(self) -> BTreeMap<String, AnalyticsValue> {
        self.columns
    }
}

/// A mandatory bound for materialized analytics query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryLimit(u32);

impl QueryLimit {
    /// Creates a result limit between 1 and 10,000 rows.
    pub fn new(limit: u32) -> Result<Self, PolyglotError> {
        if limit == 0 || limit > MAX_QUERY_ROWS {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "analytics query limit",
                reason: "limit must be between 1 and 10000",
            });
        }
        Ok(Self(limit))
    }

    /// Returns the validated row limit.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Parameterized DuckDB operations.
///
/// SQL text is an application-controlled structural input. All untrusted
/// dynamic values must be passed through `parameters`.
#[async_trait]
pub trait AnalyticsRepository: Send + Sync {
    /// Executes a parameterized statement and returns the affected row count.
    async fn execute(
        &self,
        sql: impl Into<String> + Send,
        parameters: Vec<AnalyticsValue>,
    ) -> Result<usize, PolyglotError>;

    /// Executes a parameterized query with mandatory result materialization bound.
    async fn query(
        &self,
        sql: impl Into<String> + Send,
        parameters: Vec<AnalyticsValue>,
        limit: QueryLimit,
    ) -> Result<Vec<AnalyticsRow>, PolyglotError>;
}

/// In-process DuckDB adapter with blocking work isolated from Tokio workers.
#[derive(Clone)]
pub struct DuckDbStore {
    connection: Arc<Mutex<Connection>>,
}

impl DuckDbStore {
    /// Opens an ephemeral in-memory analytics database.
    pub async fn in_memory() -> Result<Self, PolyglotError> {
        let connection = tokio::task::spawn_blocking(Connection::open_in_memory)
            .await
            .map_err(worker_error)?
            .map_err(|error| PolyglotError::driver("DuckDB", error))?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Opens or creates a DuckDB database file.
    pub async fn open(path: impl Into<PathBuf> + Send + 'static) -> Result<Self, PolyglotError> {
        let path = path.into();
        let connection = tokio::task::spawn_blocking(move || Connection::open(path))
            .await
            .map_err(worker_error)?
            .map_err(|error| PolyglotError::driver("DuckDB", error))?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Declares the adapter's bounded behavior.
    pub const fn capabilities() -> BackendCapabilities {
        BackendCapabilities::new(Backend::DuckDb, &[Capability::Analytics])
    }
}

#[async_trait]
impl AnalyticsRepository for DuckDbStore {
    async fn execute(
        &self,
        sql: impl Into<String> + Send,
        parameters: Vec<AnalyticsValue>,
    ) -> Result<usize, PolyglotError> {
        let connection = Arc::clone(&self.connection);
        let sql = sql.into();
        validate_analytics_request(&sql, &parameters)?;
        tokio::task::spawn_blocking(move || {
            let connection = connection.lock().map_err(|_| PolyglotError::Worker {
                backend: "DuckDB",
                message: "connection lock was poisoned".to_owned(),
            })?;
            let values = parameters
                .into_iter()
                .map(to_duckdb_value)
                .collect::<Result<Vec<_>, _>>()?;
            connection
                .execute(&sql, params_from_iter(values.iter()))
                .map_err(|error| PolyglotError::driver("DuckDB", error))
        })
        .await
        .map_err(worker_error)?
    }

    async fn query(
        &self,
        sql: impl Into<String> + Send,
        parameters: Vec<AnalyticsValue>,
        limit: QueryLimit,
    ) -> Result<Vec<AnalyticsRow>, PolyglotError> {
        let connection = Arc::clone(&self.connection);
        let sql = sql.into();
        validate_analytics_request(&sql, &parameters)?;
        tokio::task::spawn_blocking(move || {
            let connection = connection.lock().map_err(|_| PolyglotError::Worker {
                backend: "DuckDB",
                message: "connection lock was poisoned".to_owned(),
            })?;
            let values = parameters
                .into_iter()
                .map(to_duckdb_value)
                .collect::<Result<Vec<_>, _>>()?;
            let mut statement = connection
                .prepare(&sql)
                .map_err(|error| PolyglotError::driver("DuckDB", error))?;
            let mut rows = statement
                .query(params_from_iter(values.iter()))
                .map_err(|error| PolyglotError::driver("DuckDB", error))?;
            let Some(metadata) = rows.as_ref() else {
                return Err(PolyglotError::Driver {
                    backend: "DuckDB",
                    message: "query returned no result metadata".to_owned(),
                });
            };
            let column_names = metadata.column_names();
            let mut materialized = Vec::new();
            while materialized.len() < limit.get() as usize {
                let Some(row) = rows
                    .next()
                    .map_err(|error| PolyglotError::driver("DuckDB", error))?
                else {
                    break;
                };
                let mut columns = BTreeMap::new();
                for (index, name) in column_names.iter().enumerate() {
                    if columns.contains_key(name) {
                        return Err(PolyglotError::Driver {
                            backend: "DuckDB",
                            message: format!("duplicate result column name: {name}"),
                        });
                    }
                    let value = row
                        .get_ref(index)
                        .map_err(|error| PolyglotError::driver("DuckDB", error))?;
                    columns.insert(name.clone(), from_duckdb_value(value)?);
                }
                materialized.push(AnalyticsRow { columns });
            }
            Ok(materialized)
        })
        .await
        .map_err(worker_error)?
    }
}

fn validate_analytics_request(
    sql: &str,
    parameters: &[AnalyticsValue],
) -> Result<(), PolyglotError> {
    if sql.trim().is_empty() || sql.len() > MAX_SQL_BYTES {
        return Err(PolyglotError::InvalidIdentifier {
            kind: "DuckDB SQL statement",
            reason: "SQL must contain between 1 byte and 1 MiB",
        });
    }
    if parameters.len() > MAX_PARAMETERS {
        return Err(PolyglotError::InvalidIdentifier {
            kind: "DuckDB SQL parameters",
            reason: "a statement accepts at most 1024 parameters",
        });
    }
    let bytes = parameters.iter().try_fold(0usize, |total, value| {
        total.checked_add(match value {
            AnalyticsValue::Null => 0,
            AnalyticsValue::Boolean(_) => 1,
            AnalyticsValue::Signed(_)
            | AnalyticsValue::Unsigned(_)
            | AnalyticsValue::Decimal { .. }
            | AnalyticsValue::Float(_)
            | AnalyticsValue::Timestamp { .. }
            | AnalyticsValue::Time { .. }
            | AnalyticsValue::Interval { .. } => 16,
            AnalyticsValue::DateDays(_) => 4,
            AnalyticsValue::Text(value) => value.len(),
            AnalyticsValue::Bytes(value) | AnalyticsValue::Geometry(value) => value.len(),
        })
    });
    if bytes.is_none_or(|bytes| bytes > MAX_PARAMETER_BYTES) {
        return Err(PolyglotError::InvalidIdentifier {
            kind: "DuckDB SQL parameters",
            reason: "parameter payload must not exceed 8 MiB",
        });
    }
    Ok(())
}

fn to_duckdb_value(value: AnalyticsValue) -> Result<duckdb::types::Value, PolyglotError> {
    use duckdb::types::{Decimal, Value};

    match value {
        AnalyticsValue::Null => Ok(Value::Null),
        AnalyticsValue::Boolean(value) => Ok(Value::Boolean(value)),
        AnalyticsValue::Signed(value) => Ok(Value::HugeInt(value)),
        AnalyticsValue::Unsigned(value) => Ok(Value::UHugeInt(value)),
        AnalyticsValue::Float(value) => Ok(Value::Double(value)),
        AnalyticsValue::Text(value) => Ok(Value::Text(value)),
        AnalyticsValue::Bytes(value) => Ok(Value::Blob(value)),
        AnalyticsValue::Decimal {
            precision,
            scale,
            scaled,
        } => Decimal::new(precision, scale, scaled)
            .map(Value::Decimal)
            .map_err(|error| PolyglotError::UnsupportedValue {
                backend: "DuckDB",
                kind: error.to_string(),
            }),
        AnalyticsValue::Timestamp { unit, value } => {
            Ok(Value::Timestamp(to_duckdb_time_unit(unit), value))
        }
        AnalyticsValue::DateDays(value) => Ok(Value::Date32(value)),
        AnalyticsValue::Time { unit, value } => Ok(Value::Time64(to_duckdb_time_unit(unit), value)),
        AnalyticsValue::Interval {
            months,
            days,
            nanos,
        } => Ok(Value::Interval {
            months,
            days,
            nanos,
        }),
        AnalyticsValue::Geometry(value) => Ok(Value::Geometry(value)),
    }
}

fn from_duckdb_value(value: ValueRef<'_>) -> Result<AnalyticsValue, PolyglotError> {
    Ok(match value {
        ValueRef::Null => AnalyticsValue::Null,
        ValueRef::Boolean(value) => AnalyticsValue::Boolean(value),
        ValueRef::TinyInt(value) => AnalyticsValue::Signed(i128::from(value)),
        ValueRef::SmallInt(value) => AnalyticsValue::Signed(i128::from(value)),
        ValueRef::Int(value) => AnalyticsValue::Signed(i128::from(value)),
        ValueRef::BigInt(value) => AnalyticsValue::Signed(i128::from(value)),
        ValueRef::HugeInt(value) => AnalyticsValue::Signed(value),
        ValueRef::UTinyInt(value) => AnalyticsValue::Unsigned(u128::from(value)),
        ValueRef::USmallInt(value) => AnalyticsValue::Unsigned(u128::from(value)),
        ValueRef::UInt(value) => AnalyticsValue::Unsigned(u128::from(value)),
        ValueRef::UBigInt(value) => AnalyticsValue::Unsigned(u128::from(value)),
        ValueRef::UHugeInt(value) => AnalyticsValue::Unsigned(value),
        ValueRef::Float(value) => AnalyticsValue::Float(f64::from(value)),
        ValueRef::Double(value) => AnalyticsValue::Float(value),
        ValueRef::Decimal(value) => AnalyticsValue::Decimal {
            precision: value.width(),
            scale: value.scale(),
            scaled: value.value(),
        },
        ValueRef::Timestamp(unit, value) => AnalyticsValue::Timestamp {
            unit: from_duckdb_time_unit(unit),
            value,
        },
        ValueRef::Text(value) => AnalyticsValue::Text(
            std::str::from_utf8(value)
                .map_err(PolyglotError::serialization)?
                .to_owned(),
        ),
        ValueRef::Blob(value) => AnalyticsValue::Bytes(value.to_vec()),
        ValueRef::Geometry(value) => AnalyticsValue::Geometry(value.to_vec()),
        ValueRef::Date32(value) => AnalyticsValue::DateDays(value),
        ValueRef::Time64(unit, value) => AnalyticsValue::Time {
            unit: from_duckdb_time_unit(unit),
            value,
        },
        ValueRef::Interval {
            months,
            days,
            nanos,
        } => AnalyticsValue::Interval {
            months,
            days,
            nanos,
        },
        other => {
            return Err(PolyglotError::UnsupportedValue {
                backend: "DuckDB",
                kind: format!("{:?}", other.data_type()),
            });
        }
    })
}

fn to_duckdb_time_unit(unit: AnalyticsTimeUnit) -> duckdb::types::TimeUnit {
    match unit {
        AnalyticsTimeUnit::Second => duckdb::types::TimeUnit::Second,
        AnalyticsTimeUnit::Millisecond => duckdb::types::TimeUnit::Millisecond,
        AnalyticsTimeUnit::Microsecond => duckdb::types::TimeUnit::Microsecond,
        AnalyticsTimeUnit::Nanosecond => duckdb::types::TimeUnit::Nanosecond,
    }
}

fn from_duckdb_time_unit(unit: duckdb::types::TimeUnit) -> AnalyticsTimeUnit {
    match unit {
        duckdb::types::TimeUnit::Second => AnalyticsTimeUnit::Second,
        duckdb::types::TimeUnit::Millisecond => AnalyticsTimeUnit::Millisecond,
        duckdb::types::TimeUnit::Microsecond => AnalyticsTimeUnit::Microsecond,
        duckdb::types::TimeUnit::Nanosecond => AnalyticsTimeUnit::Nanosecond,
    }
}

fn worker_error(error: tokio::task::JoinError) -> PolyglotError {
    PolyglotError::Worker {
        backend: "DuckDB",
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests;
