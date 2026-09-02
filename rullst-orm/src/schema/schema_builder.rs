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
    pub async fn create<F>(table_name: &str, callback: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Blueprint),
    {
        validate_table_name(table_name)?;

        let mut blueprint = Blueprint::new();
        callback(&mut blueprint);

        // build() returns Result so any column-name or default issues
        // surface as errors rather than producing malformed SQL.
        let columns_sql = blueprint.build()?;

        let driver = crate::Orm::driver()?;
        if driver == "postgres" {
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
        }
        let escaped_table = match driver {
            "mysql" => format!("`{}`", table_name),
            _ => format!("\"{}\"", table_name),
        };

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (\n    {}\n);",
            escaped_table, columns_sql
        );

        let mut query_builder = sqlx::query_builder::QueryBuilder::new("");
        query_builder.push(&sql);
        let query = query_builder.build();
        crate::execute_query!(query, execute, pool)?;

        Ok(())
    }

    #[cfg_attr(mutants, mutants::skip)]
    pub async fn drop_if_exists(table_name: &str) -> Result<(), Error> {
        validate_table_name(table_name)?;
        let driver = crate::Orm::driver()?;
        let escaped_table = match driver {
            "mysql" => format!("`{}`", table_name),
            _ => format!("\"{}\"", table_name),
        };

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
    pub async fn drop_native_enum<E: DatabaseEnum>() -> Result<(), Error> {
        let definition = validated_definition::<E>()?;
        if crate::Orm::driver()? != "postgres" {
            return Ok(());
        }
        let sql = format!("DROP TYPE IF EXISTS \"{}\";", definition.type_name);
        sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
            .execute(crate::Orm::try_pool()?)
            .await?;
        Ok(())
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
    let pool = crate::Orm::try_pool()?;
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(pool)
        .await?;

    let actual = sqlx::query_as::<_, (String,)>(
        "SELECT e.enumlabel::TEXT FROM pg_type AS t JOIN pg_enum AS e ON e.enumtypid = t.oid WHERE t.typname = $1 AND pg_type_is_visible(t.oid) ORDER BY e.enumsortorder",
    )
    .bind(definition.type_name)
    .fetch_all(pool)
    .await?
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
