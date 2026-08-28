use sqlx::{AnyConnection, Connection, Row};
use std::fs;
use std::path::Path;

#[tokio::main]
pub async fn generate_models_from_db(
    driver: &str,
    url: &str,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to database...");
    sqlx::any::install_default_drivers();

    let mut conn = AnyConnection::connect(url).await?;

    println!("Introspecting schema...");

    let tables = match driver {
        "sqlite" => get_sqlite_tables(&mut conn).await?,
        "postgres" => get_postgres_tables(&mut conn).await?,
        "mysql" => get_mysql_tables(&mut conn).await?,
        _ => return Err("Unsupported driver. Use sqlite, postgres, or mysql".into()),
    };

    let out_path = Path::new(output);
    if !out_path.exists() {
        fs::create_dir_all(out_path)?;
    }

    for table in &tables {
        let columns = match driver {
            "sqlite" => get_sqlite_columns(&mut conn, table).await?,
            "postgres" => get_postgres_columns(&mut conn, table).await?,
            "mysql" => get_mysql_columns(&mut conn, table).await?,
            _ => unreachable!(),
        };

        let struct_code = generate_struct(table, &columns);

        let file_path = out_path.join(format!("{}.rs", super::model_to_snake_case(table)));
        fs::write(&file_path, struct_code)?;
        println!("Generated model for table `{}` at {:?}", table, file_path);
    }

    // Generate mod.rs
    let mod_rs = tables
        .iter()
        .map(|t| format!("pub mod {};", super::model_to_snake_case(t)))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(out_path.join("mod.rs"), mod_rs)?;

    println!("Generation complete!");

    Ok(())
}

async fn get_sqlite_tables(conn: &mut AnyConnection) -> Result<Vec<String>, sqlx::Error> {
    let query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
    let rows = sqlx::query(query).fetch_all(conn).await?;
    Ok(rows.into_iter().map(|row| row.get("name")).collect())
}

async fn get_postgres_tables(conn: &mut AnyConnection) -> Result<Vec<String>, sqlx::Error> {
    let query = "SELECT table_name FROM information_schema.tables WHERE table_schema='public'";
    let rows = sqlx::query(query).fetch_all(conn).await?;
    Ok(rows.into_iter().map(|row| row.get("table_name")).collect())
}

async fn get_mysql_tables(conn: &mut AnyConnection) -> Result<Vec<String>, sqlx::Error> {
    let query = "SELECT table_name FROM information_schema.tables WHERE table_schema=DATABASE()";
    let rows = sqlx::query(query).fetch_all(conn).await?;
    Ok(rows.into_iter().map(|row| row.get("table_name")).collect())
}

struct ColumnInfo {
    name: String,
    data_type: String,
    not_null: bool,
}

async fn get_sqlite_columns(
    conn: &mut AnyConnection,
    table: &str,
) -> Result<Vec<ColumnInfo>, sqlx::Error> {
    let query = format!("PRAGMA table_info({})", table);
    let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .fetch_all(conn)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let name: String = row.get("name");
            let typ: String = row.get("type");
            let notnull: i64 = row.get("notnull");
            ColumnInfo {
                name,
                data_type: typ,
                not_null: notnull > 0,
            }
        })
        .collect())
}

async fn get_postgres_columns(
    conn: &mut AnyConnection,
    table: &str,
) -> Result<Vec<ColumnInfo>, sqlx::Error> {
    let query = format!(
        "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}'",
        table
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .fetch_all(conn)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let name: String = row.get("column_name");
            let typ: String = row.get("data_type");
            let is_nullable: String = row.get("is_nullable");
            ColumnInfo {
                name,
                data_type: typ,
                not_null: is_nullable == "NO",
            }
        })
        .collect())
}

async fn get_mysql_columns(
    conn: &mut AnyConnection,
    table: &str,
) -> Result<Vec<ColumnInfo>, sqlx::Error> {
    let query = format!(
        "SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name = '{}' AND table_schema = DATABASE()",
        table
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .fetch_all(conn)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let name: String = row.get("column_name");
            let typ: String = row.get("data_type");
            let is_nullable: String = row.get("is_nullable");
            ColumnInfo {
                name,
                data_type: typ,
                not_null: is_nullable == "NO",
            }
        })
        .collect())
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
        "date" | "datetime" | "timestamp" | "timestamp without time zone" => "String", // Default to String for dates if chrono is not configured
        _ => "String",                                                                 // Fallback
    };

    if not_null {
        base_type.to_string()
    } else {
        format!("Option<{}>", base_type)
    }
}

fn generate_struct(table_name: &str, columns: &[ColumnInfo]) -> String {
    let struct_name = super::model_to_pascal_case(table_name);
    let mut code = String::new();
    code.push_str("use rullst_orm::{Orm, FromRow};\n");
    code.push_str("use serde::{Serialize, Deserialize};\n\n");

    code.push_str("#[derive(Clone, Debug, Serialize, Deserialize, Orm, FromRow)]\n");
    code.push_str(&format!("#[orm(table = \"{}\")]\n", table_name));
    code.push_str(&format!("pub struct {} {{\n", struct_name));

    for col in columns {
        let rust_type = map_db_type_to_rust(&col.data_type, col.not_null);
        code.push_str(&format!("    pub {}: {},\n", col.name, rust_type));
    }

    code.push_str("}\n");
    code
}
