//! `make:migration:auto` rendering: additive changes with column types
//! derived from the model field's Rust type, so SQLx can decode what the
//! migration stores, and reviewable comments for destructive differences.

use super::super::schema_diff::{ParsedField, ParsedTable};

/// How a supported field type is declared.
struct ColumnKind {
    /// `Blueprint` method used when the whole table is created.
    blueprint: &'static str,
    /// SQLite column type used when the column is added to an existing table.
    sql_type: &'static str,
    /// Literal that fills existing rows when a required column is added, or
    /// `None` when no neutral value decodes (a date or an encrypted value).
    backfill: Option<&'static str>,
}

/// Field types the auto migration can declare. Anything else (for example
/// `Vec<u8>`, `Uuid` or an application enum) is refused instead of silently
/// becoming TEXT, which SQLx cannot decode into the field.
fn column_kind(rust_type: &str) -> Option<ColumnKind> {
    let (blueprint, sql_type, backfill) = match rust_type {
        "i8" | "i16" | "i32" => ("integer", "INTEGER", Some("0")),
        "i64" => ("big_integer", "BIGINT", Some("0")),
        "f32" | "f64" => ("float", "REAL", Some("0.0")),
        "bool" => ("boolean", "INTEGER", Some("0")),
        "String" => ("string", "TEXT", Some("''")),
        "SecretString" | "NaiveDate" | "NaiveDateTime" | "NaiveTime" | "DateTime" | "Json" => {
            ("string", "TEXT", None)
        }
        _ => return None,
    };
    Some(ColumnKind {
        blueprint,
        sql_type,
        backfill,
    })
}

fn supported(table: &str, field: &ParsedField) -> Result<ColumnKind, String> {
    column_kind(&field.rust_type).ok_or_else(|| {
        // The extractor reports a tuple, array or reference type as `Unknown`.
        let rust_type = match field.rust_type.as_str() {
            "Unknown" => "a non-path type".to_string(),
            other => format!("type `{other}`"),
        };
        format!(
            "`{table}.{}` has {rust_type}, which make:migration:auto cannot map to a column (supported: i8, i16, i32, i64, f32, f64, bool, String, SecretString, chrono date/time types and Json); add it with `cargo rullst make:migration` or mark it `#[orm(skip)]`",
            field.name
        )
    })
}

/// The `Schema::create` line declaring `field` in a new table. A non-`Option`
/// field is `NOT NULL`, matching what its Rust type can decode.
fn create_column(table: &str, field: &ParsedField) -> Result<String, String> {
    let kind = supported(table, field)?;
    let constraint = if field.is_option { "" } else { ".not_null()" };
    Ok(format!(
        "            table.{}(\"{}\"){constraint};\n",
        kind.blueprint, field.name
    ))
}

/// The statement adding `field` to an existing table. A required field gets a
/// typed `NOT NULL DEFAULT` so existing rows still decode; one without a
/// neutral value is refused.
fn add_column(table: &str, field: &ParsedField) -> Result<String, String> {
    let kind = supported(table, field)?;
    let name = &field.name;
    let sql_type = kind.sql_type;
    let (comment, definition) = if field.is_option {
        (String::new(), sql_type.to_string())
    } else {
        let Some(backfill) = kind.backfill else {
            return Err(format!(
                "`{table}.{name}` is a required `{}` added to an existing table, so its rows would hold NULL; make the field `Option<{}>` or write the migration with `cargo rullst make:migration`",
                field.rust_type, field.rust_type
            ));
        };
        (
            format!(
                "        // Existing rows receive DEFAULT {backfill}; review before applying.\n"
            ),
            format!("{sql_type} NOT NULL DEFAULT {backfill}"),
        )
    };
    Ok(format!(
        "{comment}        rullst_orm::sqlx::query(\"ALTER TABLE {table} ADD COLUMN {name} {definition}\").execute(rullst_orm::Orm::pool()?).await?;\n"
    ))
}

/// Renders the reviewable auto-sync migration, or `None` when nothing differs.
/// Fails, listing every offending field, when an added column's Rust type has
/// no supported column mapping or a required column cannot be backfilled.
pub(crate) fn render_auto_migration(
    file_stem: &str,
    ast_tables: &[ParsedTable],
    db_schema: &std::collections::HashMap<String, Vec<String>>,
) -> Result<Option<String>, std::io::Error> {
    let mut up_queries = Vec::new();
    let mut down_queries = Vec::new();
    let mut problems = Vec::new();

    for ast_table in ast_tables {
        let tname = &ast_table.table_name;
        if !db_schema.contains_key(tname) {
            let mut up_sql = format!(
                "        Schema::create(\"{}\", |table| {{\n            table.id();\n",
                tname
            );
            for field in &ast_table.fields {
                if field.name == "id" || field.name == "created_at" || field.name == "updated_at" {
                    continue;
                }
                match create_column(tname, field) {
                    Ok(line) => up_sql.push_str(&line),
                    Err(problem) => problems.push(problem),
                }
            }
            // A plain literal: `format!` escaping does not apply to `push_str`.
            up_sql.push_str("            table.timestamps();\n        }).await?;\n");
            up_queries.push(up_sql);

            down_queries.push(format!(
                "        Schema::drop_if_exists(\"{}\").await?;\n",
                tname
            ));
        } else if let Some(db_cols) = db_schema.get(tname) {
            for field in &ast_table.fields {
                if !db_cols.contains(&field.name) {
                    match add_column(tname, field) {
                        Ok(statement) => up_queries.push(statement),
                        Err(problem) => problems.push(problem),
                    }
                    down_queries.push(format!("        rullst_orm::sqlx::query(\"ALTER TABLE {} DROP COLUMN {}\").execute(rullst_orm::Orm::pool()?).await?;\n", tname, field.name));
                }
            }
        }
    }

    if !problems.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "make:migration:auto wrote nothing:\n  - {}",
                problems.join("\n  - ")
            ),
        ));
    }

    for (db_tname, db_cols) in db_schema {
        if let Some(ast_table) = ast_tables.iter().find(|t| &t.table_name == db_tname) {
            for db_col in db_cols {
                if db_col == "id" || db_col == "created_at" || db_col == "updated_at" {
                    continue;
                }
                if !ast_table.fields.iter().any(|f| &f.name == db_col) {
                    up_queries.push(format!("        // WARNING: Destructive operation detected. Uncomment to apply.\n        // rullst_orm::sqlx::query(\"ALTER TABLE {} DROP COLUMN {}\").execute(rullst_orm::Orm::pool()?).await?;\n", db_tname, db_col));
                }
            }
        } else {
            up_queries.push(format!("        // WARNING: Destructive operation detected. Uncomment to apply.\n        // Schema::drop_if_exists(\"{}\").await?;\n", db_tname));
        }
    }

    if up_queries.is_empty() {
        return Ok(None);
    }
    let up_body = up_queries.join("\n");
    let down_body = down_queries.join("\n");

    Ok(Some(format!(
        r#"use rullst_orm::schema::{{Schema, Migration}};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {{
{}
        Ok(())
    }}

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {{
{}
        Ok(())
    }}
}}
"#,
        file_stem, up_body, down_body
    )))
}
