// src/generators/model.rs — Model generator.

use crate::generators::{
    is_rullst_project, migration::regenerate_migrations_mod, model_to_pascal_case,
    model_to_snake_case, pluralize,
};
use colored::*;
use std::fs;
use std::path::Path;

pub fn create_new_model(
    name: &str,
    create_migration: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validate if we are in the root of the Rullst project
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        println!(
            "{}",
            "Make sure the current folder contains a 'Cargo.toml' file with a 'rullst' dependency."
                .yellow()
        );
        std::process::exit(1);
    }

    let snake_name = model_to_snake_case(name);
    let pascal_name = model_to_pascal_case(name);
    let plural_name = pluralize(&snake_name);

    println!(
        "{}",
        format!("🛠️ Generating Rullst model: {}...", pascal_name)
            .cyan()
            .bold()
    );

    // 2. Ensure src/models directory exists
    let models_dir = Path::new("src/models");
    if !models_dir.exists() {
        fs::create_dir_all(models_dir)?;
    }

    // 3. Garantir que o src/models/mod.rs existe
    let mod_path = models_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Register new model in mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
        fs::write(&mod_path, mod_content)?;
    }

    // 5. Create model file
    let model_path = models_dir.join(format!("{}.rs", snake_name));
    if model_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Model '{}.rs' already exists. Skipping file creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = format!(
            r#"use rullst::db::{{Orm, RullstModel, FromRow, sqlx}};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "{plural_name}")]
pub struct {pascal_name} {{
    pub id: i32,
    // Add your fields here (e.g. pub name: String)
}}
"#
        );
        fs::write(&model_path, template)?;
    }

    // 6. Attempt to inject "pub mod models;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod models;") && !main_content.contains("mod models;") {
            main_content = format!("pub mod models;\n{}", main_content);
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Automatically added 'pub mod models;' to the top of src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        format!(
            "✨ Model '{}' successfully created at '{}'!",
            pascal_name,
            model_path.display()
        )
        .green()
        .bold()
    );

    // 7. Create migration if requested
    if create_migration {
        let migrations_dir = Path::new("src/migrations");
        if !migrations_dir.exists() {
            fs::create_dir_all(migrations_dir)?;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let migration_name = format!("create_{}", plural_name);
        let file_stem = format!("m{}_{}", timestamp, migration_name);
        let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

        let template = format!(
            r#"use rullst::db::schema::{{Schema, Blueprint, Migration}};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst::db::sqlx::Error> {{
        Schema::create("{plural_name}", |table| {{
            table.id();
            // Add your fields here (e.g. table.string("title");)
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst::db::sqlx::Error> {{
        Schema::drop_if_exists("{plural_name}").await
    }}
}}
"#,
            file_stem = file_stem,
            plural_name = plural_name
        );

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

        // Regenerar src/migrations/mod.rs
        regenerate_migrations_mod()?;
    }

    println!("{}", "How to import and use:".cyan());
    println!(
        "{}",
        format!(
            "  1. Use: 'use crate::models::{}::{};'",
            snake_name, pascal_name
        )
        .cyan()
    );
    println!(
        "{}",
        format!(
            "  2. Fetch data: 'let items = {}::all().await?;'",
            pascal_name
        )
        .cyan()
    );

    Ok(())
}
