//! `Schema::table`: adds columns, indexes and constraints to an existing table.

use super::Blueprint;
use super::column::ColumnDefault;
use super::constraints::{Index, IndexKind};
use super::ddl::index_statement;
use super::schema_builder::{Schema, execute_statements, prepare_postgres_enums, quoted_table};
use super::validation::validate_table_name;
use crate::Error;

impl Schema {
    /// Alters an existing table: adds the blueprint's columns
    /// (`ALTER TABLE ... ADD COLUMN`), then its unique constraints, indexes,
    /// foreign keys and CHECK constraints. Added in v13.
    ///
    /// SQLite cannot add a constraint to an existing table. There a unique
    /// constraint, or a `unique()` column, becomes the equivalent
    /// `CREATE UNIQUE INDEX`, and a foreign key on a column added in the same
    /// call becomes that column's `REFERENCES` clause (the column must then
    /// be nullable without a default). Any other SQLite foreign key, a CHECK,
    /// a primary-key column, a `NOT NULL` column without a default or a
    /// `CURRENT_TIMESTAMP` default fails with
    /// [`Error::UnsupportedSchemaChange`] before any statement runs.
    ///
    /// The statements run in order, in the task-scoped transaction when one is
    /// active; MySQL/MariaDB commit each DDL statement on its own.
    pub async fn table<F>(table_name: &str, callback: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Blueprint),
    {
        validate_table_name(table_name)?;
        let mut blueprint = Blueprint::new();
        callback(&mut blueprint);
        let driver = crate::Orm::driver()?;
        let statements =
            blueprint.alter_statements(table_name, driver, crate::database::ANY_POOL)?;
        prepare_postgres_enums(&blueprint, driver).await?;
        execute_statements(&statements).await
    }
}

fn unsupported(driver: &str, change: String) -> Error {
    Error::UnsupportedSchemaChange {
        driver: driver.to_string(),
        change,
    }
}

impl Blueprint {
    /// The `ALTER TABLE`/`CREATE INDEX` statements of [`Schema::table`],
    /// validated as a whole before any of them runs.
    pub(super) fn alter_statements(
        &self,
        table: &str,
        driver: &str,
        any_pool: bool,
    ) -> Result<Vec<String>, Error> {
        self.reject_unreadable_temporal_columns(driver, any_pool)?;
        // Validates every column, constraint and index and, on MySQL, the
        // keys on TEXT columns, exactly like `CREATE TABLE`.
        self.render(Some(table), driver, any_pool)?;
        let sqlite = driver == "sqlite";
        let quoted = quoted_table(driver, table);
        let mut statements = vec![];
        let mut sqlite_unique_columns = vec![];
        for column in &self.columns {
            // SQLite cannot add a UNIQUE column; a unique index is the same
            // constraint.
            let mut definition =
                self.column_definition(column, driver, any_pool, column.is_unique && !sqlite)?;
            if sqlite {
                self.check_sqlite_added_column(column, driver)?;
                if column.is_unique {
                    sqlite_unique_columns.push(column.name.as_str());
                }
                if let Some(foreign) = self
                    .foreign_keys
                    .iter()
                    .find(|foreign| foreign.column == column.name)
                {
                    if let Some(name) = &foreign.name {
                        super::constraints::validate_constraint_name(name)?;
                        definition.push_str(&format!(" CONSTRAINT {name}"));
                    }
                    definition.push(' ');
                    definition.push_str(&foreign.column_reference_sql(driver)?);
                }
            }
            statements.push(format!("ALTER TABLE {quoted} ADD COLUMN {definition};"));
        }
        for column in sqlite_unique_columns {
            statements.push(index_statement(
                &Index::new(&[column], IndexKind::Unique),
                table,
                driver,
            )?);
        }
        for index in &self.indexes {
            if index.kind == IndexKind::UniqueConstraint && !sqlite {
                let name = index
                    .name
                    .clone()
                    .unwrap_or_else(|| index.default_name(table));
                super::constraints::validate_constraint_name(&name)?;
                statements.push(format!(
                    "ALTER TABLE {quoted} ADD CONSTRAINT {name} UNIQUE ({});",
                    index.columns_sql()?
                ));
            } else {
                statements.push(index_statement(index, table, driver)?);
            }
        }
        for foreign in &self.foreign_keys {
            if sqlite {
                if !self
                    .columns
                    .iter()
                    .any(|column| column.name == foreign.column)
                {
                    return Err(unsupported(
                        driver,
                        format!(
                            "SQLite cannot add a foreign key to the existing column `{}`; rebuild the table with Schema::create",
                            foreign.column
                        ),
                    ));
                }
                continue;
            }
            let name = foreign
                .name
                .clone()
                .unwrap_or_else(|| foreign.default_name(table));
            super::constraints::validate_constraint_name(&name)?;
            statements.push(format!(
                "ALTER TABLE {quoted} ADD CONSTRAINT {name} {};",
                foreign.body_sql(driver)?
            ));
        }
        for check in &self.checks {
            if sqlite {
                return Err(unsupported(
                    driver,
                    "SQLite cannot add a CHECK constraint to an existing table; rebuild the table with Schema::create".to_string(),
                ));
            }
            statements.push(format!("ALTER TABLE {quoted} ADD {};", check.sql()?));
        }
        Ok(statements)
    }

    /// SQLite's `ADD COLUMN` limits, reported as a typed error instead of the
    /// database's message after earlier statements ran.
    fn check_sqlite_added_column(&self, column: &super::Column, driver: &str) -> Result<(), Error> {
        let reason = if column.is_primary_key {
            Some("a PRIMARY KEY column")
        } else if matches!(column.default_value, Some(ColumnDefault::CurrentTimestamp)) {
            Some("a column whose default is the current time")
        } else if !column.is_nullable
            && matches!(column.default_value, None | Some(ColumnDefault::Null))
        {
            Some("a NOT NULL column without a default")
        } else if self
            .foreign_keys
            .iter()
            .any(|foreign| foreign.column == column.name)
            && !matches!(column.default_value, None | Some(ColumnDefault::Null))
        {
            Some("a foreign-key column with a non-NULL default")
        } else {
            None
        };
        match reason {
            Some(reason) => Err(unsupported(
                driver,
                format!(
                    "SQLite cannot add {reason} (`{}`) to an existing table",
                    column.name
                ),
            )),
            None => Ok(()),
        }
    }
}
