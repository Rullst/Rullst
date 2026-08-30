use std::collections::BTreeMap;

use futures::TryStreamExt;
use sqlx::{Column, Row, TypeInfo, ValueRef};

use super::{PolyglotError, TursoQueryLimit, TursoRow, TursoStatement, TursoValue};

pub(super) fn to_libsql_values(values: Vec<TursoValue>) -> Vec<libsql::Value> {
    values
        .into_iter()
        .map(|value| match value {
            TursoValue::Null => libsql::Value::Null,
            TursoValue::Integer(value) => libsql::Value::Integer(value),
            TursoValue::Real(value) => libsql::Value::Real(value),
            TursoValue::Text(value) => libsql::Value::Text(value),
            TursoValue::Blob(value) => libsql::Value::Blob(value),
        })
        .collect()
}

pub(super) fn from_libsql_row(row: &libsql::Row) -> Result<TursoRow, PolyglotError> {
    let mut columns = BTreeMap::new();
    for index in 0..row.column_count() {
        let name = row
            .column_name(index)
            .ok_or_else(|| PolyglotError::Driver {
                backend: "Turso",
                message: format!("result column {index} has no name"),
            })?;
        if columns.contains_key(name) {
            return Err(PolyglotError::Driver {
                backend: "Turso",
                message: format!("duplicate result column name: {name}"),
            });
        }
        let value = row
            .get_value(index)
            .map_err(|error| PolyglotError::driver("Turso", error))?;
        columns.insert(name.to_owned(), from_libsql_value(value));
    }
    Ok(TursoRow { columns })
}

fn from_libsql_value(value: libsql::Value) -> TursoValue {
    match value {
        libsql::Value::Null => TursoValue::Null,
        libsql::Value::Integer(value) => TursoValue::Integer(value),
        libsql::Value::Real(value) => TursoValue::Real(value),
        libsql::Value::Text(value) => TursoValue::Text(value),
        libsql::Value::Blob(value) => TursoValue::Blob(value),
    }
}

pub(super) async fn execute_sqlite(
    pool: &sqlx::SqlitePool,
    statement: TursoStatement,
) -> Result<u64, PolyglotError> {
    let query = bind_sqlite(
        sqlx::query(sqlx::AssertSqlSafe(statement.sql.as_str())),
        statement.parameters,
    );
    query
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
        .map_err(|error| PolyglotError::driver("Turso offline", error))
}

pub(super) async fn transaction_sqlite(
    pool: &sqlx::SqlitePool,
    statements: Vec<TursoStatement>,
) -> Result<Vec<u64>, PolyglotError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| PolyglotError::driver("Turso offline", error))?;
    let mut affected = Vec::with_capacity(statements.len());
    for statement in statements {
        let query = bind_sqlite(
            sqlx::query(sqlx::AssertSqlSafe(statement.sql.as_str())),
            statement.parameters,
        );
        let result = query
            .execute(&mut *transaction)
            .await
            .map_err(|error| PolyglotError::driver("Turso offline", error))?;
        affected.push(result.rows_affected());
    }
    transaction
        .commit()
        .await
        .map_err(|error| PolyglotError::driver("Turso offline", error))?;
    Ok(affected)
}

pub(super) async fn query_sqlite(
    pool: &sqlx::SqlitePool,
    statement: TursoStatement,
    limit: TursoQueryLimit,
) -> Result<Vec<TursoRow>, PolyglotError> {
    let mut rows = bind_sqlite(
        sqlx::query(sqlx::AssertSqlSafe(statement.sql.as_str())),
        statement.parameters,
    )
    .fetch(pool);
    let mut materialized = Vec::new();
    while materialized.len() < limit.get() as usize {
        let Some(row) = rows
            .try_next()
            .await
            .map_err(|error| PolyglotError::driver("Turso offline", error))?
        else {
            break;
        };
        materialized.push(from_sqlite_row(&row)?);
    }
    Ok(materialized)
}

fn bind_sqlite<'query>(
    mut query: sqlx::query::Query<'query, sqlx::Sqlite, sqlx::sqlite::SqliteArguments>,
    parameters: Vec<TursoValue>,
) -> sqlx::query::Query<'query, sqlx::Sqlite, sqlx::sqlite::SqliteArguments> {
    for value in parameters {
        query = match value {
            TursoValue::Null => query.bind(Option::<i64>::None),
            TursoValue::Integer(value) => query.bind(value),
            TursoValue::Real(value) => query.bind(value),
            TursoValue::Text(value) => query.bind(value),
            TursoValue::Blob(value) => query.bind(value),
        };
    }
    query
}

fn from_sqlite_row(row: &sqlx::sqlite::SqliteRow) -> Result<TursoRow, PolyglotError> {
    let mut values = BTreeMap::new();
    for (index, column) in row.columns().iter().enumerate() {
        let name = column.name();
        if values.contains_key(name) {
            return Err(PolyglotError::Driver {
                backend: "Turso offline",
                message: format!("duplicate result column name: {name}"),
            });
        }
        let raw = row
            .try_get_raw(index)
            .map_err(|error| PolyglotError::driver("Turso offline", error))?;
        let value = if raw.is_null() {
            TursoValue::Null
        } else {
            match raw.type_info().name() {
                "INTEGER" => TursoValue::Integer(
                    row.try_get(index)
                        .map_err(|error| PolyglotError::driver("Turso offline", error))?,
                ),
                "REAL" => TursoValue::Real(
                    row.try_get(index)
                        .map_err(|error| PolyglotError::driver("Turso offline", error))?,
                ),
                "TEXT" => TursoValue::Text(
                    row.try_get(index)
                        .map_err(|error| PolyglotError::driver("Turso offline", error))?,
                ),
                "BLOB" => TursoValue::Blob(
                    row.try_get(index)
                        .map_err(|error| PolyglotError::driver("Turso offline", error))?,
                ),
                kind => {
                    return Err(PolyglotError::UnsupportedValue {
                        backend: "Turso offline",
                        kind: kind.to_owned(),
                    });
                }
            }
        };
        values.insert(name.to_owned(), value);
    }
    Ok(TursoRow { columns: values })
}
