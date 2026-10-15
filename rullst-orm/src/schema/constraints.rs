//! Indexes, unique constraints, foreign keys and CHECK constraints declared on
//! a [`Blueprint`](super::Blueprint).
//!
//! Every identifier is validated with the portable identifier grammar before
//! it reaches DDL. A CHECK expression is trusted developer input: it is
//! bounded and rejected when it could leave its parentheses, but it is never
//! built from request data.

use super::validation::validate_table_name;
use crate::Error;

/// PostgreSQL truncates longer identifiers (`NAMEDATALEN - 1`); MySQL allows 64.
const MAX_CONSTRAINT_NAME_BYTES: usize = 63;
/// Upper bound of a CHECK expression, in bytes.
pub const MAX_CHECK_EXPRESSION_BYTES: usize = 512;

/// What a foreign key does to the referencing rows when the referenced row is
/// deleted (`ON DELETE`) or its key changes (`ON UPDATE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ForeignKeyAction {
    /// `CASCADE`: delete or update the referencing rows too.
    Cascade,
    /// `RESTRICT`: reject the change while a row references it.
    Restrict,
    /// `SET NULL`: set the referencing column to `NULL`; the column must be
    /// nullable.
    SetNull,
    /// `NO ACTION`: reject the change at the end of the statement while a row
    /// references it. This is what a key without an action does.
    NoAction,
}

impl ForeignKeyAction {
    pub(super) fn sql(self) -> &'static str {
        match self {
            Self::Cascade => "CASCADE",
            Self::Restrict => "RESTRICT",
            Self::SetNull => "SET NULL",
            Self::NoAction => "NO ACTION",
        }
    }
}

/// A single-column foreign key, added with
/// [`Blueprint::foreign`](super::Blueprint::foreign) or
/// [`Blueprint::foreign_id`](super::Blueprint::foreign_id).
#[derive(Clone, Debug)]
pub struct ForeignKey {
    pub(super) column: String,
    pub(super) referenced: Option<(String, String)>,
    pub(super) on_delete: Option<ForeignKeyAction>,
    pub(super) on_update: Option<ForeignKeyAction>,
    pub(super) name: Option<String>,
}

impl ForeignKey {
    pub(super) fn new(column: String) -> Self {
        Self {
            column,
            referenced: None,
            on_delete: None,
            on_update: None,
            name: None,
        }
    }

    /// The referenced table and column, usually its `id`.
    pub fn references(&mut self, table: impl Into<String>, column: impl Into<String>) -> &mut Self {
        self.referenced = Some((table.into(), column.into()));
        self
    }

    /// The `ON DELETE` action. Without one the database rejects deleting a
    /// referenced row (`NO ACTION`).
    pub fn on_delete(&mut self, action: ForeignKeyAction) -> &mut Self {
        self.on_delete = Some(action);
        self
    }

    /// The `ON UPDATE` action.
    pub fn on_update(&mut self, action: ForeignKeyAction) -> &mut Self {
        self.on_update = Some(action);
        self
    }

    /// Names the constraint instead of `<table>_<column>_foreign`.
    pub fn name(&mut self, name: impl Into<String>) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    pub(super) fn default_name(&self, table: &str) -> String {
        format!("{table}_{}_foreign", self.column)
    }

    /// `FOREIGN KEY (column) REFERENCES table (column) ...`, without a name.
    pub(super) fn body_sql(&self, driver: &str) -> Result<String, Error> {
        validate_table_name(&self.column)?;
        let (table, column) = self.referenced.as_ref().ok_or_else(|| {
            Error::Validation(format!(
                "foreign key on `{}` needs references(table, column)",
                self.column
            ))
        })?;
        validate_table_name(table)?;
        validate_table_name(column)?;
        let mut sql = format!(
            "FOREIGN KEY ({}) {}",
            self.column,
            self.references_sql(driver, table, column)
        );
        sql.push_str(&self.actions_sql());
        Ok(sql)
    }

    /// `REFERENCES table (column)`, the column-level form SQLite accepts on
    /// `ALTER TABLE ... ADD COLUMN`.
    pub(super) fn column_reference_sql(&self, driver: &str) -> Result<String, Error> {
        let (table, column) = self.referenced.as_ref().ok_or_else(|| {
            Error::Validation(format!(
                "foreign key on `{}` needs references(table, column)",
                self.column
            ))
        })?;
        validate_table_name(table)?;
        validate_table_name(column)?;
        Ok(format!(
            "{}{}",
            self.references_sql(driver, table, column),
            self.actions_sql()
        ))
    }

    fn references_sql(&self, driver: &str, table: &str, column: &str) -> String {
        format!(
            "REFERENCES {} ({column})",
            super::schema_builder::quoted_table(driver, table)
        )
    }

    fn actions_sql(&self) -> String {
        let mut sql = String::new();
        if let Some(action) = self.on_delete {
            sql.push_str(" ON DELETE ");
            sql.push_str(action.sql());
        }
        if let Some(action) = self.on_update {
            sql.push_str(" ON UPDATE ");
            sql.push_str(action.sql());
        }
        sql
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum IndexKind {
    /// `CREATE INDEX`.
    Plain,
    /// `CREATE UNIQUE INDEX`.
    Unique,
    /// A table-level `CONSTRAINT ... UNIQUE (...)`.
    UniqueConstraint,
}

/// An index or a table-level unique constraint, added with
/// [`Blueprint::index`](super::Blueprint::index),
/// [`Blueprint::unique_index`](super::Blueprint::unique_index) or
/// [`Blueprint::unique`](super::Blueprint::unique).
#[derive(Clone, Debug)]
pub struct Index {
    pub(super) columns: Vec<String>,
    pub(super) kind: IndexKind,
    pub(super) name: Option<String>,
}

impl Index {
    pub(super) fn new(columns: &[&str], kind: IndexKind) -> Self {
        Self {
            columns: columns.iter().map(|column| column.to_string()).collect(),
            kind,
            name: None,
        }
    }

    /// Names the index or constraint instead of `<table>_<columns>_index`
    /// (`_unique` for a unique one).
    pub fn name(&mut self, name: impl Into<String>) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    pub(super) fn is_unique(&self) -> bool {
        self.kind != IndexKind::Plain
    }

    pub(super) fn default_name(&self, table: &str) -> String {
        let suffix = if self.is_unique() { "unique" } else { "index" };
        format!("{table}_{}_{suffix}", self.columns.join("_"))
    }

    /// The validated, comma-separated column list.
    pub(super) fn columns_sql(&self) -> Result<String, Error> {
        if self.columns.is_empty() {
            return Err(Error::Validation(
                "an index or unique constraint needs at least one column".to_string(),
            ));
        }
        for column in &self.columns {
            validate_table_name(column)?;
        }
        Ok(self.columns.join(", "))
    }
}

/// A table-level `CHECK` constraint, added with
/// [`Blueprint::check`](super::Blueprint::check).
#[derive(Clone, Debug)]
pub struct Check {
    pub(super) expression: String,
    pub(super) name: Option<String>,
}

impl Check {
    pub(super) fn new(expression: String) -> Self {
        Self {
            expression,
            name: None,
        }
    }

    /// Names the constraint; without a name the database chooses one.
    pub fn name(&mut self, name: impl Into<String>) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    /// `[CONSTRAINT name] CHECK (expression)`.
    pub(super) fn sql(&self) -> Result<String, Error> {
        validate_check_expression(&self.expression)?;
        let check = format!("CHECK ({})", self.expression);
        match &self.name {
            Some(name) => {
                validate_constraint_name(name)?;
                Ok(format!("CONSTRAINT {name} {check}"))
            }
            None => Ok(check),
        }
    }
}

/// Validates a constraint or index name: a portable identifier of at most 63
/// bytes, so PostgreSQL does not truncate it.
pub(super) fn validate_constraint_name(name: &str) -> Result<(), Error> {
    validate_table_name(name)?;
    if name.len() > MAX_CONSTRAINT_NAME_BYTES {
        return Err(Error::Validation(format!(
            "constraint or index name `{name}` is longer than {MAX_CONSTRAINT_NAME_BYTES} bytes; set a shorter one with .name(...)"
        )));
    }
    Ok(())
}

/// Rejects a CHECK expression that could end its `CHECK (...)` clause or hide
/// text from review: statement separators, comments, backslashes, `$`
/// (PostgreSQL dollar quoting), control characters, unbalanced parentheses
/// and unterminated quotes.
pub(super) fn validate_check_expression(expression: &str) -> Result<(), Error> {
    let invalid = |reason: &str| {
        Err(Error::Validation(format!(
            "CHECK expression {reason}; it is trusted developer SQL, not user input"
        )))
    };
    if expression.trim().is_empty() {
        return invalid("must not be empty");
    }
    if expression.len() > MAX_CHECK_EXPRESSION_BYTES {
        return invalid("must be at most 512 bytes");
    }
    if expression.contains("--") || expression.contains("/*") || expression.contains("*/") {
        return invalid("must not contain SQL comments");
    }
    if expression
        .chars()
        .any(|character| matches!(character, ';' | '\\' | '$' | '`') || character.is_control())
    {
        return invalid("must not contain `;`, `\\`, `$`, backticks or control characters");
    }
    let mut depth = 0_usize;
    let mut quote = None;
    for character in expression.chars() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth = match depth.checked_sub(1) {
                    Some(depth) => depth,
                    None => return invalid("has an unbalanced `)`"),
                };
            }
            (None, _) => {}
        }
    }
    if quote.is_some() {
        return invalid("has an unterminated quote");
    }
    if depth != 0 {
        return invalid("has an unbalanced `(`");
    }
    Ok(())
}

#[cfg(test)]
#[path = "constraints_tests.rs"]
mod tests;
