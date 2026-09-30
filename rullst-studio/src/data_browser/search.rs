//! Search predicate and row count shared by the table view.

use super::db::{
    StudioColumn, ensure_pool_initialized, fetch_table_schema, is_safe_identifier,
    sanitize_identifier,
};
use super::identifiers::{build_search_clause, qualified_table_name};
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
