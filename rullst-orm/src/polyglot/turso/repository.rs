use std::marker::PhantomData;

use super::{
    PolyglotError, TursoCodec, TursoModel, TursoQueryLimit, TursoRow, TursoStatement, TursoStore,
    TursoValue,
};

const DEFAULT_QUERY_LIMIT: u32 = 500;
const MAX_FILTERS: usize = 64;

/// Validated sort direction for a Turso model query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TursoOrder {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

impl TursoOrder {
    const fn sql(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

/// A typed repository bound to one explicit Turso store.
pub struct TursoRepository<'store, Model> {
    store: &'store TursoStore,
    marker: PhantomData<Model>,
}

impl<'store, Model> Clone for TursoRepository<'store, Model> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Model> Copy for TursoRepository<'_, Model> {}

impl<'store, Model> TursoRepository<'store, Model>
where
    Model: TursoModel,
{
    pub(super) const fn new(store: &'store TursoStore) -> Self {
        Self {
            store,
            marker: PhantomData,
        }
    }

    /// Starts a bounded, parameterized query for this model.
    pub fn query(self) -> TursoQuery<'store, Model> {
        TursoQuery::new(self.store)
    }

    /// Returns at most 10,000 records in primary-key order.
    pub async fn all(self) -> Result<Vec<Model>, PolyglotError> {
        self.query()
            .order_by(Model::primary_key_column(), TursoOrder::Asc)?
            .limit(10_000)?
            .get()
            .await
    }

    /// Finds one model by primary key.
    pub async fn find<Key>(self, key: Key) -> Result<Option<Model>, PolyglotError>
    where
        Key: TursoCodec,
    {
        self.query()
            .where_eq(Model::primary_key_column(), &key)?
            .first()
            .await
    }

    /// Inserts a new model or updates a model whose primary key is set.
    pub async fn save(self, model: &mut Model) -> Result<(), PolyglotError> {
        if model.primary_key_is_unset() {
            self.insert(model, false).await
        } else {
            self.update(model).await
        }
    }

    /// Inserts a model even when an application-assigned primary key is set.
    pub async fn create(self, model: &mut Model) -> Result<(), PolyglotError> {
        self.insert(model, !model.primary_key_is_unset()).await
    }

    /// Deletes one model by its primary key.
    pub async fn delete(self, model: &Model) -> Result<(), PolyglotError> {
        if model.primary_key_is_unset() {
            return Err(PolyglotError::NotFound);
        }
        let sql = format!(
            "DELETE FROM {} WHERE {} = ?1",
            quoted(Model::table_name()),
            quoted(Model::primary_key_column())
        );
        let affected = self
            .store
            .execute(TursoStatement::new(sql, vec![model.primary_key_value()?])?)
            .await?;
        if affected == 0 {
            return Err(PolyglotError::NotFound);
        }
        Ok(())
    }

    async fn insert(self, model: &mut Model, include_primary: bool) -> Result<(), PolyglotError> {
        let columns = Model::columns();
        let values = model.encode_turso()?;
        validate_shape(columns, &values)?;
        let primary_index = primary_index::<Model>()?;
        let insert_columns = columns
            .iter()
            .enumerate()
            .filter(|(index, _)| include_primary || *index != primary_index)
            .map(|(_, column)| quoted(column))
            .collect::<Vec<_>>();
        let parameters = values
            .into_iter()
            .enumerate()
            .filter(|(index, _)| include_primary || *index != primary_index)
            .map(|(_, value)| value)
            .collect::<Vec<_>>();
        let sql = if insert_columns.is_empty() {
            format!(
                "INSERT INTO {} DEFAULT VALUES RETURNING {}",
                quoted(Model::table_name()),
                quoted(Model::primary_key_column())
            )
        } else {
            let placeholders = (1..=insert_columns.len())
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "INSERT INTO {} ({}) VALUES ({}) RETURNING {}",
                quoted(Model::table_name()),
                insert_columns.join(", "),
                placeholders,
                quoted(Model::primary_key_column())
            )
        };
        let rows = self
            .store
            .query(
                TursoStatement::new(sql, parameters)?,
                TursoQueryLimit::new(1)?,
            )
            .await?;
        let row = rows.first().ok_or_else(|| PolyglotError::Driver {
            backend: "Turso",
            message: "insert did not return a primary key".to_owned(),
        })?;
        let value = required_cell(row, Model::primary_key_column())?;
        model.assign_primary_key(value)
    }

    async fn update(self, model: &Model) -> Result<(), PolyglotError> {
        let columns = Model::columns();
        let values = model.encode_turso()?;
        validate_shape(columns, &values)?;
        let primary_index = primary_index::<Model>()?;
        let mut parameters = Vec::with_capacity(values.len());
        let mut assignments = Vec::with_capacity(values.len().saturating_sub(1));
        for (index, (column, value)) in columns.iter().zip(values).enumerate() {
            if index == primary_index {
                continue;
            }
            parameters.push(value);
            assignments.push(format!("{} = ?{}", quoted(column), parameters.len()));
        }
        if assignments.is_empty() {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso model",
                reason: "an update requires at least one non-primary-key column",
            });
        }
        parameters.push(model.primary_key_value()?);
        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ?{}",
            quoted(Model::table_name()),
            assignments.join(", "),
            quoted(Model::primary_key_column()),
            parameters.len()
        );
        let affected = self
            .store
            .execute(TursoStatement::new(sql, parameters)?)
            .await?;
        if affected == 0 {
            return Err(PolyglotError::NotFound);
        }
        Ok(())
    }
}

/// Fluent, parameterized query builder for a typed Turso model.
pub struct TursoQuery<'store, Model> {
    store: &'store TursoStore,
    filters: Vec<(&'static str, TursoValue)>,
    order: Option<(&'static str, TursoOrder)>,
    limit: u32,
    offset: u64,
    marker: PhantomData<Model>,
}

impl<'store, Model> TursoQuery<'store, Model>
where
    Model: TursoModel,
{
    fn new(store: &'store TursoStore) -> Self {
        Self {
            store,
            filters: Vec::new(),
            order: None,
            limit: DEFAULT_QUERY_LIMIT,
            offset: 0,
            marker: PhantomData,
        }
    }

    /// Adds one equality predicate after validating the model column.
    pub fn where_eq<Value>(mut self, column: &str, value: &Value) -> Result<Self, PolyglotError>
    where
        Value: TursoCodec,
    {
        if self.filters.len() >= MAX_FILTERS {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso query",
                reason: "a query accepts at most 64 filters",
            });
        }
        let column = model_column::<Model>(column)?;
        self.filters.push((column, value.encode_turso()?));
        Ok(self)
    }

    /// Selects one validated model column for deterministic ordering.
    pub fn order_by(mut self, column: &str, order: TursoOrder) -> Result<Self, PolyglotError> {
        self.order = Some((model_column::<Model>(column)?, order));
        Ok(self)
    }

    /// Sets a mandatory 1–10,000 materialization and SQL limit.
    pub fn limit(mut self, limit: u32) -> Result<Self, PolyglotError> {
        TursoQueryLimit::new(limit)?;
        self.limit = limit;
        Ok(self)
    }

    /// Sets a checked SQL offset.
    pub fn offset(mut self, offset: u64) -> Result<Self, PolyglotError> {
        if offset > i64::MAX as u64 {
            return Err(PolyglotError::InvalidIdentifier {
                kind: "Turso query offset",
                reason: "offset must fit in a signed 64-bit integer",
            });
        }
        self.offset = offset;
        Ok(self)
    }

    /// Executes and decodes the query.
    pub async fn get(self) -> Result<Vec<Model>, PolyglotError> {
        let (sql, parameters) = self.select_sql();
        let rows = self
            .store
            .query(
                TursoStatement::new(sql, parameters)?,
                TursoQueryLimit::new(self.limit)?,
            )
            .await?;
        rows.iter().map(Model::decode_turso).collect()
    }

    /// Returns the first matching model.
    pub async fn first(mut self) -> Result<Option<Model>, PolyglotError> {
        self.limit = 1;
        self.get().await.map(|rows| rows.into_iter().next())
    }

    /// Counts rows matching the current predicates.
    pub async fn count(self) -> Result<u64, PolyglotError> {
        let (where_sql, parameters) = self.where_sql();
        let sql = format!(
            "SELECT COUNT(*) AS rullst_count FROM {}{}",
            quoted(Model::table_name()),
            where_sql
        );
        let rows = self
            .store
            .query(
                TursoStatement::new(sql, parameters)?,
                TursoQueryLimit::new(1)?,
            )
            .await?;
        match rows.first().and_then(|row| row.get("rullst_count")) {
            Some(TursoValue::Integer(value)) => {
                u64::try_from(*value).map_err(PolyglotError::serialization)
            }
            Some(other) => Err(PolyglotError::Serialization(format!(
                "expected INTEGER count, received {}",
                other.kind_name()
            ))),
            None => Err(PolyglotError::Driver {
                backend: "Turso",
                message: "count query returned no value".to_owned(),
            }),
        }
    }

    fn select_sql(&self) -> (String, Vec<TursoValue>) {
        let columns = Model::columns()
            .iter()
            .map(|column| quoted(column))
            .collect::<Vec<_>>()
            .join(", ");
        let (where_sql, parameters) = self.where_sql();
        let order_sql = self
            .order
            .map(|(column, order)| format!(" ORDER BY {} {}", quoted(column), order.sql()))
            .unwrap_or_default();
        (
            format!(
                "SELECT {columns} FROM {}{where_sql}{order_sql} LIMIT {} OFFSET {}",
                quoted(Model::table_name()),
                self.limit,
                self.offset
            ),
            parameters,
        )
    }

    fn where_sql(&self) -> (String, Vec<TursoValue>) {
        if self.filters.is_empty() {
            return (String::new(), Vec::new());
        }
        let predicates = self
            .filters
            .iter()
            .enumerate()
            .map(|(index, (column, _))| format!("{} = ?{}", quoted(column), index + 1))
            .collect::<Vec<_>>()
            .join(" AND ");
        let parameters = self
            .filters
            .iter()
            .map(|(_, value)| value.clone())
            .collect();
        (format!(" WHERE {predicates}"), parameters)
    }
}

impl TursoStore {
    /// Creates a typed model repository over this store.
    pub fn models<Model>(&self) -> TursoRepository<'_, Model>
    where
        Model: TursoModel,
    {
        TursoRepository::new(self)
    }
}

fn validate_shape(columns: &[&str], values: &[TursoValue]) -> Result<(), PolyglotError> {
    if columns.is_empty() || columns.len() != values.len() {
        return Err(PolyglotError::InvalidIdentifier {
            kind: "Turso model",
            reason: "column and encoded-value counts must match and be non-empty",
        });
    }
    Ok(())
}

fn primary_index<Model>() -> Result<usize, PolyglotError>
where
    Model: TursoModel,
{
    Model::columns()
        .iter()
        .position(|column| *column == Model::primary_key_column())
        .ok_or(PolyglotError::InvalidIdentifier {
            kind: "Turso model",
            reason: "primary key must be part of the model columns",
        })
}

fn model_column<Model>(requested: &str) -> Result<&'static str, PolyglotError>
where
    Model: TursoModel,
{
    Model::columns()
        .iter()
        .copied()
        .find(|column| *column == requested)
        .ok_or(PolyglotError::InvalidIdentifier {
            kind: "Turso model column",
            reason: "column is not declared by the model",
        })
}

fn quoted(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn required_cell<'row>(
    row: &'row TursoRow,
    column: &str,
) -> Result<&'row TursoValue, PolyglotError> {
    row.get(column).ok_or_else(|| PolyglotError::Driver {
        backend: "Turso",
        message: format!("result is missing required column {column}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_model_identifiers_are_safely_quoted() {
        assert_eq!(quoted("users"), "\"users\"");
        assert_eq!(
            quoted("users\"; DROP TABLE audit; --"),
            "\"users\"\"; DROP TABLE audit; --\""
        );
    }
}
