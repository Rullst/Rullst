//! Renders a [`Blueprint`] as `CREATE TABLE` (and index) DDL per driver.

use super::Blueprint;
use super::column::{Column, ColumnDefault, validate_text_literal};
use super::column_types::mysql_blob_like;
use super::constraints::{ForeignKeyAction, Index, IndexKind, validate_constraint_name};
use super::enums::{quoted_label, validate_native_enum};
use super::schema_builder::quoted_table;
use super::validation::validate_identifier;
use crate::Error;

impl Blueprint {
    /// The body of `CREATE TABLE`: column definitions, then table
    /// constraints and, on MySQL/MariaDB, its indexes. `table` names the
    /// constraints that default to `<table>_...`; without it the database
    /// names them.
    pub(super) fn render(
        &self,
        table: Option<&str>,
        driver: &str,
        any_pool: bool,
    ) -> Result<String, Error> {
        if let Some(column) = self.invalid_enum_columns.first() {
            return Err(Error::Validation(format!(
                "enum_col `{column}` variants must not contain backslashes or control characters"
            )));
        }
        let mut defs = vec![];
        for col in &self.columns {
            defs.push(self.column_sql(col, driver, any_pool)?);
        }
        defs.extend(self.table_constraints_sql(table, driver, any_pool)?);
        Ok(defs.join(",\n    "))
    }

    /// `CREATE TABLE IF NOT EXISTS` followed, on PostgreSQL and SQLite, by
    /// one `CREATE INDEX IF NOT EXISTS` per index.
    pub(super) fn create_statements(
        &self,
        table: &str,
        driver: &str,
        any_pool: bool,
    ) -> Result<Vec<String>, Error> {
        self.reject_unreadable_temporal_columns(driver, any_pool)?;
        let body = self.render(Some(table), driver, any_pool)?;
        let mut statements = vec![format!(
            "CREATE TABLE IF NOT EXISTS {} (\n    {}\n);",
            quoted_table(driver, table),
            body
        )];
        if driver != "mysql" {
            for index in self
                .indexes
                .iter()
                .filter(|index| index.kind != IndexKind::UniqueConstraint)
            {
                statements.push(index_statement(index, table, driver)?);
            }
        }
        Ok(statements)
    }

    /// SQLx `Any` cannot decode PostgreSQL or MySQL/MariaDB date/time
    /// columns, so a table holding one would be unreadable through it.
    pub(super) fn reject_unreadable_temporal_columns(
        &self,
        driver: &str,
        any_pool: bool,
    ) -> Result<(), Error> {
        if !any_pool || !matches!(driver, "postgres" | "mysql") {
            return Ok(());
        }
        if let Some(column) = self.columns.iter().find(|column| {
            self.driver_type(column)
                .is_some_and(|kind| kind.is_native_temporal())
        }) {
            return Err(Error::Validation(format!(
                "date/time column `{}` needs the `strict-postgres` or `strict-mysql` feature: SQLx Any cannot read PostgreSQL or MySQL/MariaDB date/time values",
                column.name
            )));
        }
        Ok(())
    }

    /// The type of `col` in `driver`'s DDL.
    fn column_type_sql(&self, col: &Column, driver: &str, any_pool: bool) -> Result<String, Error> {
        if let Some((_, definition)) = self
            .native_enum_columns
            .iter()
            .find(|(column, _)| column == &col.name)
        {
            validate_native_enum(definition.type_name, definition.variants)?;
            let labels = definition
                .variants
                .iter()
                .map(|variant| quoted_label(variant))
                .collect::<Vec<_>>()
                .join(", ");
            return Ok(match driver {
                "postgres" => format!("\"{}\"", definition.type_name),
                "mysql" => format!("ENUM({labels})"),
                _ => format!("TEXT CHECK({} IN ({labels}))", col.name),
            });
        }
        let col_type = match self.driver_type(col) {
            Some(kind) => kind.for_driver(driver, any_pool).to_string(),
            None => col.col_type.clone(),
        };
        if driver == "postgres" && col.is_auto_increment {
            if col.col_type == "INTEGER" || col.col_type == "INT" {
                return Ok("SERIAL".to_string());
            } else if col.col_type == "BIGINT" {
                return Ok("BIGSERIAL".to_string());
            }
        }
        Ok(col_type)
    }

    /// One column definition: `name TYPE [PRIMARY KEY] ... [DEFAULT x] [UNIQUE]`.
    pub(super) fn column_sql(
        &self,
        col: &Column,
        driver: &str,
        any_pool: bool,
    ) -> Result<String, Error> {
        self.column_definition(col, driver, any_pool, col.is_unique)
    }

    /// [`Blueprint::column_sql`], with the `UNIQUE` clause only when `unique`.
    pub(super) fn column_definition(
        &self,
        col: &Column,
        driver: &str,
        any_pool: bool,
        unique: bool,
    ) -> Result<String, Error> {
        // Defensive re-validation: column names must always be safe
        // identifiers regardless of how the Column was constructed.
        validate_identifier(&col.name)?;
        let col_type_str = self.column_type_sql(col, driver, any_pool)?;
        let mut def = format!("{} {}", col.name, col_type_str);
        if col.is_primary_key {
            def.push_str(" PRIMARY KEY");
        }
        if col.is_auto_increment {
            if driver == "sqlite" {
                def.push_str(" AUTOINCREMENT");
            } else if driver == "mysql" {
                def.push_str(" AUTO_INCREMENT");
            }
        }
        if !col.is_nullable && !col.is_primary_key {
            def.push_str(" NOT NULL");
        }
        if let Some(default) = &col.default_value {
            use std::fmt::Write;
            if let ColumnDefault::Text(text) = default {
                validate_text_literal(text, &format!("DEFAULT of column `{}`", col.name))?;
            }
            let default_sql = match self.driver_type(col) {
                Some(kind) => kind.default_sql(driver, any_pool, default, &col.name)?,
                None => default.to_sql(),
            };
            if driver == "mysql"
                && *default != ColumnDefault::Null
                && mysql_blob_like(&col_type_str)
            {
                let _ = write!(def, " DEFAULT ({default_sql})");
            } else {
                let _ = write!(def, " DEFAULT {default_sql}");
            }
        }
        if unique {
            def.push_str(" UNIQUE");
        }
        Ok(def)
    }

    /// Table-level unique, foreign-key and CHECK constraints, plus the
    /// MySQL/MariaDB `INDEX` clauses.
    fn table_constraints_sql(
        &self,
        table: Option<&str>,
        driver: &str,
        any_pool: bool,
    ) -> Result<Vec<String>, Error> {
        if driver == "mysql" {
            self.reject_mysql_blob_keys(any_pool)?;
        }
        let mut defs = vec![];
        for index in self
            .indexes
            .iter()
            .filter(|index| index.kind == IndexKind::UniqueConstraint)
        {
            let columns = index.columns_sql()?;
            defs.push(
                match constraint_name(index.name.as_deref(), table, |table| {
                    index.default_name(table)
                })? {
                    Some(name) => format!("CONSTRAINT {name} UNIQUE ({columns})"),
                    None => format!("UNIQUE ({columns})"),
                },
            );
        }
        for foreign in &self.foreign_keys {
            self.reject_set_null_on_required_column(&foreign.column, foreign.on_delete)?;
            self.reject_set_null_on_required_column(&foreign.column, foreign.on_update)?;
            let body = foreign.body_sql(driver)?;
            defs.push(
                match constraint_name(foreign.name.as_deref(), table, |table| {
                    foreign.default_name(table)
                })? {
                    Some(name) => format!("CONSTRAINT {name} {body}"),
                    None => body,
                },
            );
        }
        for check in &self.checks {
            defs.push(check.sql()?);
        }
        if driver == "mysql" {
            for index in self
                .indexes
                .iter()
                .filter(|index| index.kind != IndexKind::UniqueConstraint)
            {
                let unique = if index.is_unique() { "UNIQUE " } else { "" };
                let columns = index.columns_sql()?;
                defs.push(
                    match constraint_name(index.name.as_deref(), table, |table| {
                        index.default_name(table)
                    })? {
                        Some(name) => format!("{unique}INDEX {name} ({columns})"),
                        None => format!("{unique}INDEX ({columns})"),
                    },
                );
            }
        }
        Ok(defs)
    }

    /// MySQL/MariaDB reject a key on a `TEXT`/`BLOB`/`JSON` column without a
    /// prefix length, and a prefix would make a unique key compare only the
    /// prefix, so such a key fails before any DDL runs.
    fn reject_mysql_blob_keys(&self, any_pool: bool) -> Result<(), Error> {
        let keyed = self
            .columns
            .iter()
            .filter(|column| column.is_unique)
            .map(|column| column.name.as_str())
            .chain(
                self.indexes
                    .iter()
                    .flat_map(|index| index.columns.iter().map(String::as_str)),
            )
            .chain(
                self.foreign_keys
                    .iter()
                    .map(|foreign| foreign.column.as_str()),
            );
        for name in keyed {
            let Some(column) = self.columns.iter().rev().find(|column| column.name == name) else {
                continue;
            };
            if mysql_blob_like(&self.column_type_sql(column, "mysql", any_pool)?) {
                return Err(Error::Validation(format!(
                    "MySQL/MariaDB cannot index the TEXT column `{name}`; give it a bounded type such as `.col_type = \"VARCHAR(255)\".to_string()`"
                )));
            }
        }
        Ok(())
    }

    /// MySQL/MariaDB reject `SET NULL` on a `NOT NULL` column when the table
    /// is created; PostgreSQL and SQLite would fail on the first delete.
    fn reject_set_null_on_required_column(
        &self,
        column: &str,
        action: Option<ForeignKeyAction>,
    ) -> Result<(), Error> {
        let required = self
            .columns
            .iter()
            .rev()
            .find(|candidate| candidate.name == column)
            .is_some_and(|candidate| !candidate.is_nullable);
        if required && action == Some(ForeignKeyAction::SetNull) {
            return Err(Error::Validation(format!(
                "foreign key `{column}` uses SET NULL but the column is NOT NULL"
            )));
        }
        Ok(())
    }
}

/// The explicit name, else `default(table)` when the table is known.
fn constraint_name(
    explicit: Option<&str>,
    table: Option<&str>,
    default: impl FnOnce(&str) -> String,
) -> Result<Option<String>, Error> {
    let name = match (explicit, table) {
        (Some(name), _) => Some(name.to_string()),
        (None, Some(table)) => Some(default(table)),
        (None, None) => None,
    };
    if let Some(name) = &name {
        validate_constraint_name(name)?;
    }
    Ok(name)
}

/// `CREATE [UNIQUE] INDEX` for PostgreSQL and SQLite (`IF NOT EXISTS`, like
/// the table) or MySQL/MariaDB (which has no `IF NOT EXISTS` for indexes).
pub(super) fn index_statement(index: &Index, table: &str, driver: &str) -> Result<String, Error> {
    let name = constraint_name(index.name.as_deref(), Some(table), |table| {
        index.default_name(table)
    })?
    .unwrap_or_default();
    let unique = if index.is_unique() { "UNIQUE " } else { "" };
    let if_not_exists = if driver == "mysql" {
        ""
    } else {
        "IF NOT EXISTS "
    };
    Ok(format!(
        "CREATE {unique}INDEX {if_not_exists}{name} ON {} ({});",
        quoted_table(driver, table),
        index.columns_sql()?
    ))
}
