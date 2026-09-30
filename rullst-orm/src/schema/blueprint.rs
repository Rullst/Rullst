use super::column::{Column, ColumnDefault, validate_text_literal};
use super::enums::{DatabaseEnum, NativeEnumDefinition, quoted_label, validate_native_enum};
use super::validation::validate_identifier;
use crate::Error;

pub struct Blueprint {
    pub columns: Vec<Column>,
    native_enum_columns: Vec<(String, NativeEnumDefinition)>,
    /// `enum_col` columns with a variant that is not a safe DDL literal;
    /// reported by `build` because `enum_col` itself cannot fail.
    invalid_enum_columns: Vec<String>,
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
        }
    }

    pub fn id(&mut self) -> &mut Column {
        self.columns.push(Column {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            is_nullable: false,
            is_primary_key: true,
            is_auto_increment: true,
            default_value: None,
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

    pub fn float(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "REAL")
    }

    pub fn boolean(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "INTEGER")
    }

    pub fn vector(&mut self, name: &str, dimensions: usize) -> &mut Column {
        let col_type = format!("VECTOR({})", dimensions);
        self.add_column(name, &col_type)
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

    #[cfg_attr(test, mutants::skip)]
    pub fn build(&self) -> Result<String, Error> {
        self.build_for_driver(crate::Orm::driver()?)
    }

    pub(crate) fn build_for_driver(&self, driver: &str) -> Result<String, Error> {
        if let Some(column) = self.invalid_enum_columns.first() {
            return Err(Error::Validation(format!(
                "enum_col `{column}` variants must not contain backslashes or control characters"
            )));
        }
        let mut defs = vec![];
        for col in &self.columns {
            // Defensive re-validation: column names must always be safe
            // identifiers regardless of how the Column was constructed.
            validate_identifier(&col.name)?;

            let mut col_type_str = if let Some((_, definition)) = self
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
                match driver {
                    "postgres" => format!("\"{}\"", definition.type_name),
                    "mysql" => format!("ENUM({labels})"),
                    _ => format!("TEXT CHECK({} IN ({labels}))", col.name),
                }
            } else {
                col.col_type.clone()
            };
            if driver == "postgres" && col.is_auto_increment {
                if col.col_type == "INTEGER" || col.col_type == "INT" {
                    col_type_str = "SERIAL".to_string();
                } else if col.col_type == "BIGINT" {
                    col_type_str = "BIGSERIAL".to_string();
                }
            }

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
                if driver == "mysql"
                    && *default != ColumnDefault::Null
                    && mysql_requires_expression_default(&col_type_str)
                {
                    let _ = write!(def, " DEFAULT ({})", default.to_sql());
                } else {
                    let _ = write!(def, " DEFAULT {}", default.to_sql());
                }
            }
            defs.push(def);
        }
        Ok(defs.join(",\n    "))
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

/// MySQL accepts a DEFAULT on BLOB, TEXT, GEOMETRY and JSON columns only as a
/// parenthesized expression, even for a literal value (MySQL 8.0.13+).
/// MariaDB 10.2.1+ accepts the same expression form.
fn mysql_requires_expression_default(col_type: &str) -> bool {
    let base = col_type
        .split(|character: char| !character.is_ascii_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(
        base.as_str(),
        "TEXT"
            | "TINYTEXT"
            | "MEDIUMTEXT"
            | "LONGTEXT"
            | "BLOB"
            | "TINYBLOB"
            | "MEDIUMBLOB"
            | "LONGBLOB"
            | "JSON"
            | "GEOMETRY"
    )
}
