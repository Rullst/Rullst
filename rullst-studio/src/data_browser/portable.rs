//! Driver-portable execution of Studio's dynamic `QueryBuilder` statements.

use sqlx::{Database, Execute, QueryBuilder, query::Query};

/// A built Studio statement for the configured ORM database type.
pub(crate) type StudioQuery<'q> =
    Query<'q, rullst_orm::RullstDatabase, <rullst_orm::RullstDatabase as Database>::Arguments>;

/// Builds a dynamic statement whose bind markers suit the active driver.
///
/// Under the default `sqlx::Any` build, `QueryBuilder` writes `?` markers and
/// the Any PostgreSQL driver forwards the SQL unchanged, so PostgreSQL would
/// reject it. For PostgreSQL the markers are renumbered to `$n` in textual
/// order, which is also the order in which the builder recorded the values.
/// Studio's dynamic SQL holds only validated identifiers and fixed literals, so
/// every `?` is a bind marker. A `strict-postgres` build already writes `$n`,
/// which passes through unchanged.
pub(crate) fn build_for_driver<'q>(
    builder: &'q mut QueryBuilder<rullst_orm::RullstDatabase>,
    driver: &str,
) -> Result<StudioQuery<'q>, sqlx::Error> {
    if driver != "postgres" {
        return Ok(builder.build());
    }
    let sql = rullst_orm::replace_placeholders(builder.sql().as_str());
    let arguments = builder
        .build()
        .take_arguments()
        .map_err(sqlx::Error::Encode)?
        .unwrap_or_default();
    Ok(sqlx::query_with(sqlx::AssertSqlSafe(sql), arguments))
}

#[cfg(test)]
#[cfg(not(any(
    feature = "strict-postgres",
    feature = "strict-mysql",
    feature = "strict-sqlite"
)))]
mod tests {
    use super::*;
    use sqlx::Arguments;

    fn paged_search() -> QueryBuilder<rullst_orm::RullstDatabase> {
        let mut builder = QueryBuilder::new("SELECT \"id\" FROM \"records\" WHERE ");
        builder.push("CAST(\"name\" AS TEXT) ILIKE ");
        builder.push_bind("%a?b%".to_string());
        builder.push(" AND kind = 'x?' LIMIT ");
        builder.push_bind(25_i64);
        builder.push(" OFFSET ");
        builder.push_bind(50_i64);
        builder
    }

    #[test]
    fn postgres_statements_use_numbered_markers_in_binding_order() {
        let mut builder = paged_search();
        let mut query = build_for_driver(&mut builder, "postgres").expect("portable statement");
        let arguments = query
            .take_arguments()
            .expect("encodable arguments")
            .expect("prepared arguments");
        assert_eq!(arguments.len(), 3);
        assert_eq!(
            query.sql().as_str(),
            "SELECT \"id\" FROM \"records\" WHERE CAST(\"name\" AS TEXT) ILIKE $1 \
             AND kind = 'x?' LIMIT $2 OFFSET $3"
        );
    }

    #[test]
    fn other_drivers_keep_the_builder_statement() {
        for driver in ["sqlite", "mysql"] {
            let mut builder = paged_search();
            let query = build_for_driver(&mut builder, driver).expect("statement");
            assert!(
                query.sql().as_str().ends_with("LIMIT ? OFFSET ?"),
                "{driver}"
            );
        }
    }
}
