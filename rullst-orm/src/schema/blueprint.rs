use super::column::{Column, ColumnDefault, validate_text_literal};
use super::column_types::DriverType;
use super::constraints::{Check, ForeignKey, Index, IndexKind};
use super::enums::{DatabaseEnum, NativeEnumDefinition, validate_native_enum};
use crate::Error;

pub struct Blueprint {
    pub columns: Vec<Column>,
    pub(super) native_enum_columns: Vec<(String, NativeEnumDefinition)>,
    /// `enum_col` columns with a variant that is not a safe DDL literal;
    /// reported by `build` because `enum_col` itself cannot fail.
    pub(super) invalid_enum_columns: Vec<String>,
    /// Columns whose DDL type is chosen per driver when the schema is built.
    driver_typed_columns: Vec<(String, DriverType)>,
    pub(super) indexes: Vec<Index>,
    pub(super) foreign_keys: Vec<ForeignKey>,
    pub(super) checks: Vec<Check>,
}

impl Default for Blueprint {
    fn default() -> Self {
        Self::new()
    }
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            columns: vec![],
            native_enum_columns: vec![],
            invalid_enum_columns: vec![],
            driver_typed_columns: vec![],
            indexes: vec![],
            foreign_keys: vec![],
            checks: vec![],
        }
    }

    fn add_driver_typed_column(&mut self, name: &str, kind: DriverType) -> &mut Column {
        self.driver_typed_columns.push((name.to_string(), kind));
        self.add_column(name, kind.portable())
    }

    /// The driver-specific type of a column added by a typed helper, unless
    /// the caller has since replaced its `col_type`.
    pub(super) fn driver_type(&self, col: &Column) -> Option<DriverType> {
        self.driver_typed_columns
            .iter()
            .rev()
            .find(|(name, _)| name == &col.name)
            .map(|(_, kind)| *kind)
            .filter(|kind| col.col_type == kind.portable())
    }

    pub fn id(&mut self) -> &mut Column {
        self.columns.push(Column {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            is_nullable: false,
            is_primary_key: true,
            is_auto_increment: true,
            default_value: None,
            is_unique: false,
        });
        let len = self.columns.len();
        &mut self.columns[len - 1]
    }

    fn add_column(&mut self, name: &str, col_type: &str) -> &mut Column {
        let col = Column::new(name, col_type);
        self.columns.push(col);
        let len = self.columns.len();
        &mut self.columns[len - 1]
    }

    pub fn string(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "TEXT")
    }

    pub fn integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "INTEGER")
    }

    pub fn big_integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "BIGINT")
    }

    /// Adds an `f64` column: `DOUBLE PRECISION` on PostgreSQL, `DOUBLE` on
    /// MySQL/MariaDB and `REAL` (8-byte) on SQLite.
    ///
    /// [`Column::col_type`] reads `REAL` until the schema is built; replacing
    /// it keeps the replacement on every driver.
    pub fn float(&mut self, name: &str) -> &mut Column {
        self.add_driver_typed_column(name, DriverType::Double)
    }

    /// Adds a `bool` column: `BOOLEAN` on PostgreSQL and an `INTEGER` 0/1
    /// flag on MySQL/MariaDB and SQLite.
    ///
    /// On PostgreSQL `ColumnDefault::Integer(0)`/`(1)` render as
    /// `FALSE`/`TRUE`; any other integer default fails the build.
    /// [`Column::col_type`] reads `INTEGER` until the schema is built;
    /// replacing it keeps the replacement on every driver.
    pub fn boolean(&mut self, name: &str) -> &mut Column {
        self.add_driver_typed_column(name, DriverType::Boolean)
    }

    pub fn vector(&mut self, name: &str, dimensions: usize) -> &mut Column {
        let col_type = format!("VECTOR({})", dimensions);
        self.add_column(name, &col_type)
    }

    /// Adds a calendar-date column for a `chrono::NaiveDate` field: `DATE` on
    /// PostgreSQL and MySQL/MariaDB, ISO-8601 `TEXT` (`YYYY-MM-DD`) on SQLite.
    ///
    /// PostgreSQL and MySQL/MariaDB date/time columns need a strict driver
    /// feature (`strict-postgres`, `strict-mysql`): `Schema::create` rejects
    /// them on the SQLx `Any` pool, which cannot read them. A
    /// `ColumnDefault::CurrentTimestamp` default is today's UTC date.
    /// Added in v13.
    pub fn date(&mut self, name: &str) -> &mut Column {
        self.add_driver_typed_column(name, DriverType::Date)
    }

    /// Adds a time-of-day column for a `chrono::NaiveTime` field: `TIME` on
    /// PostgreSQL, `TIME(6)` on MySQL/MariaDB and `TEXT` (`HH:MM:SS[.f]`) on
    /// SQLite. See [`Blueprint::date`] for the driver requirement. Added in v13.
    pub fn time(&mut self, name: &str) -> &mut Column {
        self.add_driver_typed_column(name, DriverType::Time)
    }

    /// Adds a date-and-time column without a zone for a
    /// `chrono::NaiveDateTime` field: `TIMESTAMP` on PostgreSQL,
    /// `DATETIME(6)` on MySQL/MariaDB and `TEXT` (`YYYY-MM-DD HH:MM:SS[.f]`)
    /// on SQLite. A `ColumnDefault::CurrentTimestamp` default is the current
    /// UTC wall time. See [`Blueprint::date`] for the driver requirement.
    /// Added in v13.
    pub fn timestamp(&mut self, name: &str) -> &mut Column {
        self.add_driver_typed_column(name, DriverType::Timestamp)
    }

    /// Adds a UTC-instant column for a `chrono::DateTime<Utc>` field:
    /// `TIMESTAMPTZ` on PostgreSQL, `DATETIME(6)` holding UTC on
    /// MySQL/MariaDB (whose `TIMESTAMP` ends in 2038; SQLx sets the session
    /// time zone to `+00:00`) and RFC 3339 `TEXT`
    /// (`YYYY-MM-DDTHH:MM:SS[.f]+00:00`, which sorts as text) on SQLite. See
    /// [`Blueprint::date`] for the driver requirement. Added in v13.
    pub fn timestamp_tz(&mut self, name: &str) -> &mut Column {
        self.add_driver_typed_column(name, DriverType::TimestampTz)
    }

    /// Adds a `TEXT` column restricted to `variants` by a `CHECK` constraint.
    ///
    /// Single quotes in a variant are doubled. A variant containing a
    /// backslash or a control character makes [`Blueprint::build`] fail.
    pub fn enum_col(&mut self, name: &str, variants: Vec<&str>) -> &mut Column {
        if variants
            .iter()
            .any(|variant| validate_text_literal(variant, name).is_err())
        {
            self.invalid_enum_columns.push(name.to_string());
        }
        // Enforce enum values using a CHECK constraint for safe cross-DB compatibility
        let check_clause = variants
            .iter()
            .map(|v| format!("'{}'", v.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        let col_type = format!("TEXT CHECK({} IN ({}))", name, check_clause);
        self.add_column(name, &col_type)
    }

    /// Adds a database-native enum column from a closed [`DatabaseEnum`]
    /// contract.
    ///
    /// PostgreSQL uses the named enum type (created and drift-checked by
    /// [`Schema::create`](super::Schema::create)); MySQL/MariaDB use an inline
    /// `ENUM`, and SQLite uses `TEXT CHECK`. Metadata is validated when the
    /// blueprint is built, so a manually implemented invalid contract fails
    /// before any table DDL is executed.
    pub fn native_enum<E: DatabaseEnum>(&mut self, name: &str) -> &mut Column {
        self.native_enum_columns.push((
            name.to_string(),
            NativeEnumDefinition {
                type_name: E::TYPE_NAME,
                variants: E::VARIANTS,
            },
        ));
        self.add_column(name, "TEXT")
    }

    /// Adds nullable `created_at`/`updated_at` text columns that default to the
    /// database's current timestamp.
    ///
    /// SQLite and PostgreSQL receive `TEXT DEFAULT CURRENT_TIMESTAMP`.
    /// MySQL/MariaDB reject a literal default on a `TEXT` column, so they
    /// receive the expression form `TEXT DEFAULT (CURRENT_TIMESTAMP)`
    /// (MySQL 8.0.13+, MariaDB 10.2.1+). The column stays `TEXT` on every
    /// driver so SQLx's `Any` driver can decode it as a string.
    pub fn timestamps(&mut self) {
        let mut created = Column::new("created_at", "TEXT");
        created.default(ColumnDefault::CurrentTimestamp);
        self.columns.push(created);

        let mut updated = Column::new("updated_at", "TEXT");
        updated.default(ColumnDefault::CurrentTimestamp);
        self.columns.push(updated);
    }

    pub fn soft_deletes(&mut self) {
        let col = Column::new("deleted_at", "TEXT");
        self.columns.push(col);
        let len = self.columns.len();
        self.columns[len - 1].nullable();
    }

    /// Adds a table-level `UNIQUE (columns)` constraint, named
    /// `<table>_<columns>_unique` unless [`Index::name`] sets one. It is
    /// part of `CREATE TABLE`; MySQL/MariaDB cannot make a `TEXT` column
    /// unique (see [`Column::unique`]). Added in v13.
    pub fn unique(&mut self, columns: &[&str]) -> &mut Index {
        self.add_index(columns, IndexKind::UniqueConstraint)
    }

    /// Adds an index named `<table>_<columns>_index` unless [`Index::name`]
    /// sets one: `CREATE INDEX IF NOT EXISTS` after `CREATE TABLE` on
    /// PostgreSQL and SQLite, an `INDEX` clause of `CREATE TABLE` on
    /// MySQL/MariaDB, which cannot index a `TEXT` column. Added in v13.
    pub fn index(&mut self, columns: &[&str]) -> &mut Index {
        self.add_index(columns, IndexKind::Plain)
    }

    /// Like [`Blueprint::index`] for a `UNIQUE` index, named
    /// `<table>_<columns>_unique` by default. Added in v13.
    pub fn unique_index(&mut self, columns: &[&str]) -> &mut Index {
        self.add_index(columns, IndexKind::Unique)
    }

    fn add_index(&mut self, columns: &[&str], kind: IndexKind) -> &mut Index {
        self.indexes.push(Index::new(columns, kind));
        let len = self.indexes.len();
        &mut self.indexes[len - 1]
    }

    /// Adds a foreign key on an existing column of this blueprint, named
    /// `<table>_<column>_foreign` unless [`ForeignKey::name`] sets one:
    ///
    /// ```rust,no_run
    /// # use rullst_orm::schema::{Blueprint, ForeignKeyAction};
    /// # fn up(table: &mut Blueprint) {
    /// table.integer("user_id").not_null();
    /// table
    ///     .foreign("user_id")
    ///     .references("users", "id")
    ///     .on_delete(ForeignKeyAction::Cascade);
    /// # }
    /// ```
    ///
    /// The column type must match the referenced one (`id()` is `INTEGER`).
    /// SQLite enforces foreign keys only with `PRAGMA foreign_keys = ON`,
    /// which SQLx enables on every connection it opens. Added in v13.
    pub fn foreign(&mut self, column: impl Into<String>) -> &mut ForeignKey {
        self.foreign_keys.push(ForeignKey::new(column.into()));
        let len = self.foreign_keys.len();
        &mut self.foreign_keys[len - 1]
    }

    /// Adds an `INTEGER NOT NULL` column and a foreign key from it to
    /// `table(id)`, the key of [`Blueprint::id`]:
    /// `table.foreign_id("user_id", "users").on_delete(ForeignKeyAction::Cascade)`.
    /// For a nullable key, declare the column with [`Blueprint::integer`] and
    /// use [`Blueprint::foreign`]. Added in v13.
    pub fn foreign_id(
        &mut self,
        column: impl Into<String>,
        table: impl Into<String>,
    ) -> &mut ForeignKey {
        let column = column.into();
        self.integer(&column).not_null();
        let foreign = self.foreign(column);
        foreign.references(table, "id");
        foreign
    }

    /// Adds a table-level `CHECK (expression)` constraint.
    ///
    /// The expression is trusted developer SQL: it is emitted as written, so
    /// never build it from request data. Building fails when it is empty,
    /// longer than 512 bytes, contains `;`, a comment, `\`, `$`, a backtick
    /// or a control character, or has unbalanced parentheses or quotes.
    /// MySQL enforces CHECK from 8.0.16 and MariaDB from 10.2.1; older
    /// servers parse and ignore it. Added in v13.
    pub fn check(&mut self, expression: impl Into<String>) -> &mut Check {
        self.checks.push(Check::new(expression.into()));
        let len = self.checks.len();
        &mut self.checks[len - 1]
    }

    /// The column definitions and table constraints of a `CREATE TABLE`
    /// statement for the connected driver. Indexes that PostgreSQL and SQLite
    /// create with separate statements are not included; constraint names
    /// that default to `<table>_...` are left to the database.
    #[cfg_attr(test, mutants::skip)]
    pub fn build(&self) -> Result<String, Error> {
        self.build_for_driver(crate::Orm::driver()?)
    }

    pub(crate) fn build_for_driver(&self, driver: &str) -> Result<String, Error> {
        self.render(None, driver, crate::database::ANY_POOL)
    }

    pub(super) fn postgres_enum_definitions(&self) -> Result<Vec<NativeEnumDefinition>, Error> {
        let mut definitions = std::collections::BTreeMap::new();
        for (_, definition) in &self.native_enum_columns {
            validate_native_enum(definition.type_name, definition.variants)?;
            if let Some(existing) = definitions.get(definition.type_name) {
                if existing != definition {
                    return Err(Error::Validation(format!(
                        "database enum type `{}` has conflicting label definitions",
                        definition.type_name
                    )));
                }
            } else {
                definitions.insert(definition.type_name, definition.clone());
            }
        }
        Ok(definitions.into_values().collect())
    }
}
