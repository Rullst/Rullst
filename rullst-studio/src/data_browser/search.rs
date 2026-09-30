//! Search predicate, row count and page order of the table view.

use super::db::{
    StudioColumn, StudioTableSchema, ensure_pool_initialized, fetch_table_schema,
    is_safe_identifier, sanitize_identifier,
};
use super::identifiers::{build_search_clause, qualified_table_name, quote_table_name};
use super::limits::MAX_SEARCH_BYTES;
use super::portable::build_for_driver;
use sqlx::{QueryBuilder, Row};

/// Appends ` WHERE <column> LIKE ? OR ...` for a non-empty search term, binding
/// the term once per column. A page and its count use this same predicate
/// over the same inspected columns, so the two always agree.
pub(crate) fn push_search_predicate(
    query: &mut QueryBuilder<rullst_orm::RullstDatabase>,
    driver: &str,
    columns: &[StudioColumn],
    search: &str,
) {
    if search.is_empty() || columns.is_empty() {
        return;
    }
    query.push(" WHERE ");
    let mut separated = query.separated(" OR ");
    for column in columns {
        separated.push(build_search_clause(driver, &column.name));
        separated.push_bind_unseparated(format!("%{search}%"));
    }
}

/// Appends an `ORDER BY` so that `LIMIT`/`OFFSET` pages neither repeat nor
/// skip rows; without it PostgreSQL and MySQL return rows in any order, and a
/// PostgreSQL update moves its row within a sequential scan. Pages follow the
/// complete primary key when it is known, otherwise every selected column. Key
/// columns are qualified with the table so that they name the stored values,
/// not the text projections that the page selects under the same names.
pub(crate) fn push_page_order(
    query: &mut QueryBuilder<rullst_orm::RullstDatabase>,
    driver: &str,
    clean_table: &str,
    schema: &StudioTableSchema,
) {
    let table = qualified_table_name(driver, clean_table);
    let key_columns = schema
        .columns
        .iter()
        .filter(|column| column.primary_key)
        .map(|column| format!("{table}.{}", quote_table_name(driver, &column.name)))
        .collect::<Vec<_>>();
    let order = if schema.primary_key_complete && !key_columns.is_empty() {
        key_columns
    } else {
        (1..=schema.columns.len())
            .map(|position| position.to_string())
            .collect()
    };
    if !order.is_empty() {
        query.push(" ORDER BY ");
        query.push(order.join(", "));
    }
}

/// Counts the rows of a validated table that match [`push_search_predicate`].
pub(crate) async fn count_matching_rows(
    pool: &rullst_orm::RullstPool,
    driver: &str,
    clean_table: &str,
    columns: &[StudioColumn],
    search: &str,
) -> Result<usize, sqlx::Error> {
    let mut query = QueryBuilder::<rullst_orm::RullstDatabase>::new(format!(
        "SELECT COUNT(*) FROM {}",
        qualified_table_name(driver, clean_table)
    ));
    push_search_predicate(&mut query, driver, columns, search);
    let row = build_for_driver(&mut query, driver)?
        .fetch_one(pool)
        .await?;
    let count = row.try_get::<i64, _>(0)?;
    Ok(usize::try_from(count).unwrap_or(0))
}

/// Counts a table's rows, optionally filtered by a search term of at most
/// 256 bytes that is bound once per inspected column. The columns are those
/// the table view displays (at most 256); a failed inspection is an error
/// rather than an unfiltered count.
pub async fn count_table_rows(
    table: &str,
    search_query: Option<&str>,
) -> Result<usize, sqlx::Error> {
    if search_query.is_some_and(|search| search.len() > MAX_SEARCH_BYTES) {
        return Err(sqlx::Error::Configuration(
            "Studio search terms are limited to 256 bytes".into(),
        ));
    }
    let pool = ensure_pool_initialized().await?;
    let driver = rullst_core::db::safe_driver().unwrap_or("sqlite");
    let clean_table = sanitize_identifier(table);
    if clean_table != table || !is_safe_identifier(&clean_table) {
        return Err(sqlx::Error::Configuration(
            "Studio received an unsupported SQL identifier".into(),
        ));
    }
    let search = search_query.unwrap_or_default();
    let columns = if search.is_empty() {
        Vec::new()
    } else {
        fetch_table_schema(pool, driver, &clean_table)
            .await?
            .columns
    };
    count_matching_rows(pool, driver, &clean_table, &columns, search).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_browser::db::StudioColumnKind;
    use sqlx::Execute;

    fn schema(key: &[bool], primary_key_complete: bool) -> StudioTableSchema {
        StudioTableSchema {
            columns: key
                .iter()
                .enumerate()
                .map(|(index, primary_key)| StudioColumn {
                    name: format!("c{index}"),
                    kind: StudioColumnKind::Integer,
                    primary_key: *primary_key,
                    nullable: false,
                })
                .collect(),
            primary_key_complete,
        }
    }

    fn order(driver: &str, schema: &StudioTableSchema) -> String {
        let mut query = QueryBuilder::<rullst_orm::RullstDatabase>::new("SELECT 1");
        push_page_order(&mut query, driver, "records", schema);
        query.build().sql().as_str().to_string()
    }

    #[test]
    fn pages_follow_the_primary_key_or_every_selected_column() {
        let composite = schema(&[true, false, true], true);
        assert_eq!(
            order("postgres", &composite),
            "SELECT 1 ORDER BY \"public\".\"records\".\"c0\", \"public\".\"records\".\"c2\""
        );
        assert_eq!(
            order("mysql", &composite),
            "SELECT 1 ORDER BY `records`.`c0`, `records`.`c2`"
        );
        assert_eq!(
            order("sqlite", &composite),
            "SELECT 1 ORDER BY \"records\".\"c0\", \"records\".\"c2\""
        );
        // A missing or incomplete key cannot order rows uniquely on its own.
        assert_eq!(
            order("sqlite", &schema(&[false, false], true)),
            "SELECT 1 ORDER BY 1, 2"
        );
        assert_eq!(
            order("sqlite", &schema(&[true, false], false)),
            "SELECT 1 ORDER BY 1, 2"
        );
        assert_eq!(order("sqlite", &schema(&[], true)), "SELECT 1");
    }
}
