use super::blueprint::Blueprint;
use super::enums::{DatabaseEnum, validated_definition};
#[cfg(feature = "strict-postgres")]
use super::enums::{NativeEnumDefinition, quoted_label};
#[cfg(feature = "strict-postgres")]
use super::validation::validate_identifier;
use super::validation::validate_table_name;
use crate::Error;

pub struct Schema;

impl Schema {
    /// Creates `table_name` from the blueprint (`CREATE TABLE IF NOT EXISTS`),
    /// then, on PostgreSQL and SQLite, its indexes (`CREATE INDEX IF NOT
    /// EXISTS`). Every statement is built and validated before the first one
    /// runs; they run in the task-scoped transaction when one is active.
    pub async fn create<F>(table_name: &str, callback: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Blueprint),
    {
        validate_table_name(table_name)?;

        let mut blueprint = Blueprint::new();
        callback(&mut blueprint);

        // Building returns Result so any column-name or default issues
        // surface as errors rather than producing malformed SQL.
        let driver = crate::Orm::driver()?;
        let statements =
            blueprint.create_statements(table_name, driver, crate::database::ANY_POOL)?;
        prepare_postgres_enums(&blueprint, driver).await?;
        execute_statements(&statements).await
    }

    #[cfg_attr(mutants, mutants::skip)]
    pub async fn drop_if_exists(table_name: &str) -> Result<(), Error> {
        validate_table_name(table_name)?;
        let driver = crate::Orm::driver()?;
        let escaped_table = quoted_table(driver, table_name);

        let sql = format!("DROP TABLE IF EXISTS {};", escaped_table);
        let mut query_builder = sqlx::query_builder::QueryBuilder::new("");
        query_builder.push(&sql);
        let query = query_builder.build();
        crate::execute_query!(query, execute, pool)?;
        Ok(())
    }

    /// Drops a named PostgreSQL enum after all dependent tables have been
    /// removed. MySQL/MariaDB and SQLite have no standalone enum object, so
    /// this operation is a validated no-op for those drivers.
    ///
    /// Like the table DDL, it runs in the task-scoped transaction when one is
    /// active, so it can follow `drop_if_exists` in the same transaction and
    /// rolls back with it.
    pub async fn drop_native_enum<E: DatabaseEnum>() -> Result<(), Error> {
        let definition = validated_definition::<E>()?;
        if crate::Orm::driver()? != "postgres" {
            return Ok(());
        }
        let sql = format!("DROP TYPE IF EXISTS \"{}\";", definition.type_name);
        let query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
        crate::execute_query!(query, execute, pool)?;
        Ok(())
    }
}

/// Creates (or drift-checks) the PostgreSQL enum types a blueprint uses.
pub(super) async fn prepare_postgres_enums(
    blueprint: &Blueprint,
    driver: &str,
) -> Result<(), Error> {
    if driver != "postgres" {
        return Ok(());
    }
    let definitions = blueprint.postgres_enum_definitions()?;
    #[cfg(not(feature = "strict-postgres"))]
    if !definitions.is_empty() {
        return Err(Error::Validation(
            "PostgreSQL native enums require the `strict-postgres` feature because SQLx Any cannot decode custom PostgreSQL types"
                .to_string(),
        ));
    }
    #[cfg(feature = "strict-postgres")]
    for definition in definitions {
        ensure_postgres_enum(&definition).await?;
    }
    Ok(())
}

/// Runs validated DDL statements in order.
pub(super) async fn execute_statements(statements: &[String]) -> Result<(), Error> {
    for sql in statements {
        let query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
        crate::execute_query!(query, execute, pool)?;
    }
    Ok(())
}

/// Quotes a validated table name for the DDL dialect. PostgreSQL folds the
/// unquoted names that generated model SQL uses to lower case, so the quoted
/// DDL name is folded the same way and `Schema::create("UserProfiles")`
/// creates the `userprofiles` relation that `SELECT * FROM UserProfiles`
/// resolves to. Quoting still admits reserved words such as `order`.
pub(super) fn quoted_table(driver: &str, table_name: &str) -> String {
    match driver {
        "mysql" => format!("`{table_name}`"),
        "postgres" => format!("\"{}\"", table_name.to_ascii_lowercase()),
        _ => format!("\"{table_name}\""),
    }
}

#[cfg(feature = "strict-postgres")]
async fn ensure_postgres_enum(definition: &NativeEnumDefinition) -> Result<(), Error> {
    validate_identifier(definition.type_name)?;
    let labels = definition
        .variants
        .iter()
        .map(|variant| quoted_label(variant))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "DO $rullst$ BEGIN CREATE TYPE \"{}\" AS ENUM ({}); EXCEPTION WHEN duplicate_object THEN NULL; END $rullst$;",
        definition.type_name, labels
    );
    // The type, its drift check and the table that uses it share the
    // task-scoped transaction when one is active.
    let create = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
    crate::execute_query!(create, execute, pool)?;

    let labels = sqlx::query_as::<_, (String,)>(
        "SELECT e.enumlabel::TEXT FROM pg_type AS t JOIN pg_enum AS e ON e.enumtypid = t.oid WHERE t.typname = $1 AND pg_type_is_visible(t.oid) ORDER BY e.enumsortorder",
    )
    .bind(definition.type_name);
    let actual = crate::execute_query!(labels, fetch_all, pool)?
        .into_iter()
        .map(|(label,)| label)
        .collect::<Vec<_>>();
    if actual != definition.variants {
        return Err(Error::Validation(format!(
            "PostgreSQL enum `{}` differs from the declared Rust enum",
            definition.type_name
        )));
    }
    Ok(())
}

#[cfg(test)]
mod quoting_tests {
    use super::quoted_table;

    #[test]
    fn postgres_table_ddl_matches_unquoted_generated_sql() {
        assert_eq!(quoted_table("postgres", "UserProfiles"), "\"userprofiles\"");
        assert_eq!(quoted_table("postgres", "order"), "\"order\"");
        assert_eq!(quoted_table("mysql", "UserProfiles"), "`UserProfiles`");
        assert_eq!(quoted_table("sqlite", "UserProfiles"), "\"UserProfiles\"");
    }
}
