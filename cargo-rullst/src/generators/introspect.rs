use proc_macro2::Span;
use quote::ToTokens;
use sqlx::{AnyConnection, Connection, Row};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum IntrospectionError {
    #[error("unsupported database driver `{0}`; use sqlite, postgres, or mysql")]
    UnsupportedDriver(String),
    #[error("database introspection failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("could not write generated models: {0}")]
    Io(#[from] std::io::Error),
    #[error("database {kind} name `{name}` cannot produce a safe Rust identifier")]
    InvalidIdentifier { kind: &'static str, name: String },
    #[error(
        "database column `{name}` would require generated Rust field `{generated}`, but ORM column remapping is not supported; rename the column before generating models"
    )]
    UnsupportedColumnMapping { name: String, generated: String },
    #[error(
        "database {kind} names `{first}` and `{second}` both normalize to `{generated}`; rename one before generating models"
    )]
    IdentifierCollision {
        kind: &'static str,
        first: String,
        second: String,
        generated: String,
    },
}

struct TablePlan {
    database_name: String,
    module_name: String,
    struct_name: String,
}

#[tokio::main]
pub async fn generate_models_from_db(
    driver: &str,
    url: &str,
    output: &str,
) -> Result<(), IntrospectionError> {
    println!("Connecting to database...");
    sqlx::any::install_default_drivers();
    let mut connection = AnyConnection::connect(url).await?;

    println!("Introspecting schema...");
    let tables = match driver {
        "sqlite" => get_sqlite_tables(&mut connection).await?,
        "postgres" => get_postgres_tables(&mut connection).await?,
        "mysql" => get_mysql_tables(&mut connection).await?,
        other => return Err(IntrospectionError::UnsupportedDriver(other.to_string())),
    };
    let table_plans = plan_tables(&tables)?;
    let mut generated_models = Vec::with_capacity(table_plans.len());
    for table in &table_plans {
        let columns = match driver {
            "sqlite" => get_sqlite_columns(&mut connection, &table.database_name).await?,
            "postgres" => get_postgres_columns(&mut connection, &table.database_name).await?,
            "mysql" => get_mysql_columns(&mut connection, &table.database_name).await?,
            other => return Err(IntrospectionError::UnsupportedDriver(other.to_string())),
        };
        let struct_code = generate_struct(table, &columns)?;
        generated_models.push((table, struct_code));
    }

    // Complete metadata, identifier and code-generation validation before the
    // first filesystem mutation so one unsupported table cannot leave a
    // partially generated model directory.
    let output_path = Path::new(output);
    fs::create_dir_all(output_path)?;
    for (table, struct_code) in generated_models {
        let file_path = output_path.join(format!("{}.rs", table.module_name));
        fs::write(&file_path, struct_code)?;
        println!(
            "Generated model for table `{}` at {:?}",
            table.database_name, file_path
        );
    }

    let modules = table_plans
        .iter()
        .map(|table| format!("pub mod {};", table.module_name))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(output_path.join("mod.rs"), modules)?;
    println!("Generation complete!");
    Ok(())
}

async fn get_sqlite_tables(connection: &mut AnyConnection) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(connection)
    .await?;
    Ok(rows.into_iter().map(|row| row.get("name")).collect())
}

async fn get_postgres_tables(connection: &mut AnyConnection) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE' ORDER BY table_name",
    )
    .fetch_all(connection)
    .await?;
    Ok(rows.into_iter().map(|row| row.get("table_name")).collect())
}

async fn get_mysql_tables(connection: &mut AnyConnection) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE' ORDER BY table_name",
    )
    .fetch_all(connection)
    .await?;
    Ok(rows.into_iter().map(|row| row.get("table_name")).collect())
}

#[derive(Debug, PartialEq, Eq)]
struct ColumnInfo {
    name: String,
    data_type: String,
    not_null: bool,
}

async fn get_sqlite_columns(
    connection: &mut AnyConnection,
    table: &str,
) -> Result<Vec<ColumnInfo>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT name, type, \"notnull\" AS is_not_null, pk \
         FROM pragma_table_info(?) ORDER BY cid",
    )
    .bind(table)
    .fetch_all(connection)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let not_null: i64 = row.get("is_not_null");
            let primary_key: i64 = row.get("pk");
            ColumnInfo {
                name: row.get("name"),
                data_type: row.get("type"),
                not_null: not_null > 0 || primary_key > 0,
            }
        })
        .collect())
}

async fn get_postgres_columns(
    connection: &mut AnyConnection,
    table: &str,
) -> Result<Vec<ColumnInfo>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT column_name, data_type, is_nullable FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(connection)
    .await?;
    Ok(map_information_schema_columns(rows))
}

async fn get_mysql_columns(
    connection: &mut AnyConnection,
    table: &str,
) -> Result<Vec<ColumnInfo>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT column_name, data_type, is_nullable FROM information_schema.columns \
         WHERE table_name = ? AND table_schema = DATABASE() ORDER BY ordinal_position",
    )
    .bind(table)
    .fetch_all(connection)
    .await?;
    Ok(map_information_schema_columns(rows))
}

fn map_information_schema_columns(rows: Vec<sqlx::any::AnyRow>) -> Vec<ColumnInfo> {
    rows.into_iter()
        .map(|row| {
            let is_nullable: String = row.get("is_nullable");
            ColumnInfo {
                name: row.get("column_name"),
                data_type: row.get("data_type"),
                not_null: is_nullable == "NO",
            }
        })
        .collect()
}

fn plan_tables(tables: &[String]) -> Result<Vec<TablePlan>, IntrospectionError> {
    let mut generated_names = HashMap::new();
    tables
        .iter()
        .map(|table| {
            validate_database_identifier(table, "table")?;
            let module_name = safe_snake_identifier(table, "table", "table")?;
            if let Some(first) = generated_names.insert(module_name.clone(), table.clone()) {
                return Err(IntrospectionError::IdentifierCollision {
                    kind: "table",
                    first,
                    second: table.clone(),
                    generated: module_name,
                });
            }
            let struct_name = snake_to_pascal(&module_name);
            if !super::is_valid_rust_identifier(&struct_name) {
                return Err(IntrospectionError::InvalidIdentifier {
                    kind: "table",
                    name: table.clone(),
                });
            }
            Ok(TablePlan {
                database_name: table.clone(),
                module_name,
                struct_name,
            })
        })
        .collect()
}

fn validate_database_identifier(value: &str, kind: &'static str) -> Result<(), IntrospectionError> {
    let bytes = value.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 64
        && (bytes[0].is_ascii_alphabetic() || bytes[0] == b'_')
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(IntrospectionError::InvalidIdentifier {
            kind,
            name: value.to_string(),
        })
    }
}

fn safe_snake_identifier(
    value: &str,
    prefix: &str,
    kind: &'static str,
) -> Result<String, IntrospectionError> {
    let mut output = String::new();
    let mut separator_pending = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if character.is_ascii_uppercase()
                && !output.is_empty()
                && !output.ends_with('_')
                && output
                    .chars()
                    .last()
                    .is_some_and(|previous| previous.is_ascii_lowercase())
            {
                output.push('_');
            }
            if separator_pending && !output.is_empty() && !output.ends_with('_') {
                output.push('_');
            }
            separator_pending = false;
            output.push(character.to_ascii_lowercase());
        } else {
            separator_pending = true;
        }
    }
    let mut output = output.trim_matches('_').to_string();
    if output.is_empty() {
        return Err(IntrospectionError::InvalidIdentifier {
            kind,
            name: value.to_string(),
        });
    }
    if output.starts_with(|character: char| character.is_ascii_digit()) {
        output = format!("{prefix}_{output}");
    }
    if !super::is_valid_rust_identifier(&output) {
        output.push_str("_field");
    }
    if !super::is_valid_rust_identifier(&output) {
        return Err(IntrospectionError::InvalidIdentifier {
            kind,
            name: value.to_string(),
        });
    }
    Ok(output)
}

fn snake_to_pascal(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), characters.as_str()),
                None => String::new(),
            }
        })
        .collect()
}

fn rust_string_literal(value: &str) -> String {
    syn::LitStr::new(value, Span::call_site())
        .to_token_stream()
        .to_string()
}

fn map_db_type_to_rust(db_type: &str, not_null: bool) -> String {
    let base_type = match db_type.to_lowercase().as_str() {
        "int" | "integer" | "int4" | "serial" => "i32",
        "bigint" | "int8" | "bigserial" => "i64",
        "smallint" | "int2" => "i16",
        "tinyint" => "i8",
        "real" | "float4" => "f32",
        "double" | "float8" | "double precision" | "numeric" | "decimal" => "f64",
        "boolean" | "bool" => "bool",
        "text" | "varchar" | "char" | "character varying" | "longtext" => "String",
        "blob" | "bytea" => "Vec<u8>",
        "date" | "datetime" | "timestamp" | "timestamp without time zone" => "String",
        _ => "String",
    };
    if not_null {
        base_type.to_string()
    } else {
        format!("Option<{base_type}>")
    }
}

fn generate_struct(
    table: &TablePlan,
    columns: &[ColumnInfo],
) -> Result<String, IntrospectionError> {
    validate_database_identifier(&table.database_name, "table")?;
    let mut code =
        String::from("use rullst_orm::{FromRow, Orm};\nuse serde::{Deserialize, Serialize};\n\n");
    code.push_str("#[derive(Clone, Debug, Serialize, Deserialize, Orm, FromRow)]\n");
    code.push_str(&format!(
        "#[orm(table = {})]\n",
        rust_string_literal(&table.database_name)
    ));
    code.push_str(&format!("pub struct {} {{\n", table.struct_name));

    let mut generated_fields = HashSet::new();
    let mut first_database_name_by_field = HashMap::new();
    for column in columns {
        validate_database_identifier(&column.name, "column")?;
        let field_name = safe_snake_identifier(&column.name, "field", "column")?;
        if !generated_fields.insert(field_name.clone()) {
            return Err(IntrospectionError::IdentifierCollision {
                kind: "column",
                first: first_database_name_by_field
                    .get(&field_name)
                    .cloned()
                    .unwrap_or_default(),
                second: column.name.clone(),
                generated: field_name,
            });
        }
        first_database_name_by_field.insert(field_name.clone(), column.name.clone());
        if field_name != column.name {
            return Err(IntrospectionError::UnsupportedColumnMapping {
                name: column.name.clone(),
                generated: field_name,
            });
        }
        let rust_type = map_db_type_to_rust(&column.data_type, column.not_null);
        code.push_str(&format!("    pub {field_name}: {rust_type},\n"));
    }
    code.push_str("}\n");
    Ok(code)
}

#[cfg(test)]
#[path = "introspect_tests.rs"]
mod tests;
