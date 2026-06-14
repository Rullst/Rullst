// src/generators/middleware.rs — Middleware generator.

use crate::generators::{is_rullst_project, middleware_to_snake_case};
use colored::*;
use std::fs;
use std::path::Path;

pub fn create_new_middleware(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_project_root()?;

    let snake_name = middleware_to_snake_case(name);

    println!(
        "{}",
        format!("🛠️ Generating Rullst middleware: {}...", snake_name)
            .cyan()
            .bold()
    );

    let middlewares_dir = Path::new("src/middlewares");
    if !middlewares_dir.exists() {
        fs::create_dir_all(middlewares_dir)?;
    }

    register_middleware_in_mod(middlewares_dir, &snake_name)?;
    create_middleware_file(middlewares_dir, &snake_name)?;
    inject_middleware_module_in_main()?;

    println!(
        "{}",
        format!(
            "✨ Middleware '{}' successfully created at '{}/{}.rs'!",
            snake_name,
            middlewares_dir.display(),
            snake_name
        )
        .green()
        .bold()
    );
    println!("{}", "How to map in your routes using Axum layers:".cyan());
    println!("{}", "  1. Use: 'use rullst::server::from_fn;'".cyan());
    println!(
        "{}",
        format!(
            "  2. Use: 'use crate::middlewares::{}::{};'",
            snake_name, snake_name
        )
        .cyan()
    );
    println!(
        "{}",
        format!(
            "  3. Add: '.layer(from_fn({}))' on your router.",
            snake_name
        )
        .cyan()
    );

    Ok(())
}

fn validate_project_root() -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

fn register_middleware_in_mod(middlewares_dir: &Path, snake_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mod_path = middlewares_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

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
    Ok(())
}

fn create_middleware_file(middlewares_dir: &Path, snake_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let middleware_path = middlewares_dir.join(format!("{}.rs", snake_name));
    if middleware_path.exists() {
        println!(
            "{}",
            format!(
                "⚠️ Warning: Middleware '{}.rs' already exists. Skipping file creation.",
                snake_name
            )
            .yellow()
        );
    } else {
        let template = format!(
            r#"use rullst::server::{{Request, Next, Response}};

pub async fn {}(req: Request, next: Next) -> Response {{
    // Pre-request logic here
    
    let response = next.run(req).await;
    
    // Post-request logic here
    
    response
}}
"#,
            snake_name
        );
        fs::write(&middleware_path, template)?;
    }
    Ok(())
}

fn inject_middleware_module_in_main() -> Result<(), Box<dyn std::error::Error>> {
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod middlewares;")
            && !main_content.contains("mod middlewares;")
        {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod middlewares;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content = main_content
                    .replace("pub mod models;", "pub mod models;\npub mod middlewares;");
            } else {
                main_content = format!("pub mod middlewares;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Adicionado 'pub mod middlewares;' ao src/main.rs automaticamente.".cyan()
            );
        }
    }
    Ok(())
}
