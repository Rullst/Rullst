use super::blueprint::Blueprint;
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

        let driver = crate::Orm::driver();
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

    #[mutants::skip]
    pub async fn drop_if_exists(table_name: &str) -> Result<(), Error> {
        validate_table_name(table_name)?;
        let driver = crate::Orm::driver();
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
}
