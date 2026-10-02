// src/generators/migration.rs — Migration generator.
#![cfg_attr(mutants, mutants::skip)]

use crate::generators::{
    ProjectOrmBackend, introspect::validate_database_identifier, is_rullst_project,
    project_orm_backend,
};
use colored::*;
use std::fs;
use std::path::Path;

pub fn create_new_migration(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }

    let snake_name = migration_snake_name(name)?;
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_{}", timestamp, snake_name);

    println!(
        "{}",
        format!("🛠️ Gerando migração Rullst: {}...", file_stem)
            .cyan()
            .bold()
    );

    let migrations_dir = Path::new("src/migrations");
    if !migrations_dir.exists() {
        fs::create_dir_all(migrations_dir)?;
    }

    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));
    let table_name = get_table_name_from_migration(&snake_name);

    let template = render_migration(&file_stem, &table_name, project_orm_backend());

    fs::write(&migration_path, template)?;
    println!(
        "{}",
        format!(
            "✨ Rust migration successfully created at '{}'!",
            migration_path.display()
        )
        .green()
        .bold()
    );

    regenerate_migrations_mod()?;

    Ok(())
}

/// Normalizes a migration name without dropping any of its characters and
/// rejects names that cannot form the `m<timestamp>_<name>` module identifier.
fn migration_snake_name(name: &str) -> Result<String, std::io::Error> {
    let snake_name = name.to_lowercase().replace('-', "_");
    if is_migration_module_suffix(&snake_name) {
        Ok(snake_name)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "migration name '{name}' must contain only ASCII letters, digits, '_' or '-' (for example add_index_v2)"
            ),
        ))
    }
}

fn is_migration_module_suffix(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Whether a `src/migrations` file stem can be declared as `pub mod <stem>;`.
fn is_migration_module_name(stem: &str) -> bool {
    stem.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && crate::generators::is_valid_rust_identifier(stem)
}

pub(crate) fn render_migration(
    file_stem: &str,
    table_name: &str,
    backend: ProjectOrmBackend,
) -> String {
    match backend {
        ProjectOrmBackend::Sqlx => render_sqlx_migration(file_stem, table_name),
        ProjectOrmBackend::Turso => render_turso_migration(file_stem, table_name),
    }
}

fn render_sqlx_migration(file_stem: &str, table_name: &str) -> String {
    format!(
        r#"use rullst_orm::schema::{{Schema, Migration}};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::create("{table_name}", |table| {{
            table.id();
            // Add your fields here (e.g. table.string("title");)
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::drop_if_exists("{table_name}").await
    }}
}}
"#,
        file_stem = file_stem,
        table_name = table_name
    )
}

fn render_turso_migration(file_stem: &str, table_name: &str) -> String {
    format!(
        r#"use rullst_orm::polyglot::{{
    PolyglotError, TursoMigration, TursoStatement,
}};

pub fn migration() -> Result<TursoMigration, PolyglotError> {{
    TursoMigration::new(
        "{file_stem}",
        vec![TursoStatement::new(
            "CREATE TABLE {table_name} (id INTEGER PRIMARY KEY AUTOINCREMENT)",
            vec![],
        )?],
    )?
    .with_down(vec![TursoStatement::new(
        "DROP TABLE {table_name}",
        vec![],
    )?])
}}
"#
    )
}

fn get_table_name_from_migration(name: &str) -> String {
    let s = name.to_lowercase();
    if let Some(stripped) = s.strip_prefix("create_") {
        if let Some(inner) = stripped.strip_suffix("_table") {
            inner.to_string()
        } else {
            stripped.to_string()
        }
    } else {
        "table_name".to_string()
    }
}

pub fn regenerate_migrations_mod() -> Result<(), Box<dyn std::error::Error>> {
    let migrations_dir = Path::new("src/migrations");
    if !migrations_dir.exists() {
        return Ok(());
    }

    let paths = fs::read_dir(migrations_dir)?;
    let mut modules = vec![];
    for path in paths {
        let path = path?.path();
        if let Some(ext) = path.extension()
            && ext == "rs"
            && let Some(stem) = path.file_stem()
        {
            let stem_str = stem.to_string_lossy().to_string();
            if stem_str != "mod" && stem_str.starts_with('m') {
                // A stem such as `m..._add_index.v2` would make mod.rs unparsable.
                if !is_migration_module_name(&stem_str) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "'{}' is not a valid migration module name; rename it to ASCII letters, digits and '_'",
                            path.display()
                        ),
                    )
                    .into());
                }
                modules.push(stem_str);
            }
        }
    }
    modules.sort();

    let mut mod_content = String::new();
    mod_content.push_str("// Generated by Rullst. Do not edit manually.\n\n");
    for m in &modules {
        mod_content.push_str(&format!("pub mod {};\n", m));
    }
    match project_orm_backend() {
        ProjectOrmBackend::Sqlx => {
            mod_content.push_str(
                "\npub fn get_migrations() -> Vec<Box<dyn rullst_orm::schema::Migration>> {\n",
            );
            mod_content.push_str("    vec![\n");
            for m in &modules {
                mod_content.push_str(&format!("        Box::new({}::MigrationImpl),\n", m));
            }
            mod_content.push_str("    ]\n");
        }
        ProjectOrmBackend::Turso => {
            mod_content.push_str(
                "\npub fn get_migrations() -> Result<\n    Vec<rullst_orm::polyglot::TursoMigration>,\n    rullst_orm::polyglot::PolyglotError,\n> {\n",
            );
            mod_content.push_str("    Ok(vec![\n");
            for m in &modules {
                mod_content.push_str(&format!("        {}::migration()?,\n", m));
            }
            mod_content.push_str("    ])\n");
        }
    }
    mod_content.push_str("}\n");

    fs::write(migrations_dir.join("mod.rs"), mod_content)?;
    Ok(())
}

pub async fn create_auto_migration() -> Result<(), Box<dyn std::error::Error>> {
    use colored::*;
    use std::fs;
    use std::path::Path;

    if !crate::generators::is_rullst_project() {
        return Err(crate::ui::error_report::ProjectRequired.into());
    }

    if project_orm_backend() == ProjectOrmBackend::Turso {
        println!(
            "{}",
            "Automatic schema diff is not available for Turso-primary projects. Use `cargo rullst make:migration <name>` and review the generated reversible SQL."
                .yellow()
        );
        return Ok(());
    }

    let dotenv_path = Path::new(".env");
    if dotenv_path.exists() {
        let env_content = fs::read_to_string(dotenv_path)?;
        let mut db_url = String::new();
        for line in env_content.lines() {
            if line.starts_with("DATABASE_URL=") {
                db_url = line
                    .replace("DATABASE_URL=", "")
                    .replace("\"", "")
                    .trim()
                    .to_string();
                break;
            }
        }

        if db_url.is_empty() || !db_url.starts_with("sqlite:") {
            println!(
                "{}",
                "?? Auto migrations currently only support SQLite. Skipping auto-sync.".yellow()
            );
            return Ok(());
        }

        println!(
            "{}",
            "?? Analysing AST and Database Schema...".cyan().bold()
        );

        use sqlx::{Row, SqlitePool};
        let pool = SqlitePool::connect(&db_url).await?;

        let mut db_tables = vec![];
        let rows = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&pool)
        .await?;
        for row in rows {
            let table_name: String = row.get("name");
            validate_database_identifier(&table_name, "table")?;
            db_tables.push(table_name);
        }

        let mut db_schema = std::collections::HashMap::new();
        for table in db_tables {
            let mut cols = vec![];
            let pragma_rows = sqlx::query("SELECT name FROM pragma_table_info(?) ORDER BY cid")
                .bind(&table)
                .fetch_all(&pool)
                .await?;
            for prow in pragma_rows {
                let cname: String = prow.get("name");
                validate_database_identifier(&cname, "column")?;
                cols.push(cname);
            }
            db_schema.insert(table, cols);
        }

        let ast_tables = super::schema_diff::extract_tables_from_ast();
        for table in &ast_tables {
            validate_database_identifier(&table.table_name, "table")?;
            for field in &table.fields {
                validate_database_identifier(&field.name, "column")?;
            }
        }

        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let file_stem = format!("m{}_{}", timestamp, "auto_sync");
        let Some(template) = render_auto_migration(&file_stem, &ast_tables, &db_schema)? else {
            let destructive = destructive_differences(&ast_tables, &db_schema);
            if destructive.is_empty() {
                println!("{}", "? Database is already in sync with AST!".green());
            } else {
                println!(
                    "{}",
                    "No additive changes. These database objects have no model; drop them only in a reviewed `cargo rullst make:migration`:"
                        .yellow()
                );
                for (table, column) in destructive {
                    match column {
                        Some(column) => println!("  - column {table}.{column}"),
                        None => println!("  - table {table}"),
                    }
                }
            }
            return Ok(());
        };

        let migrations_dir = Path::new("src/migrations");
        if !migrations_dir.exists() {
            fs::create_dir_all(migrations_dir)?;
        }
        let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

        fs::write(&migration_path, template)?;
        println!(
            "{}",
            format!(
                "? Auto-migration created safely at '{}'!",
                migration_path.display()
            )
            .green()
            .bold()
        );

        crate::generators::migration::regenerate_migrations_mod()?;
    } else {
        println!(
            "{}",
            "? No .env file found. Auto migrations require a DATABASE_URL.".red()
        );
    }

    Ok(())
}

#[path = "migration_auto.rs"]
mod auto;
pub(crate) use auto::{destructive_differences, render_auto_migration};

#[cfg(test)]
#[path = "migration_tests.rs"]
mod tests;
