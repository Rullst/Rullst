//! Command-line argument translator and Artisan CLI dispatcher.

use crate::artisan::studio_server::start_studio_server;
use rullst_orm::Seeder;
use rullst_orm::schema::{Migration, run_artisan_with_args};
use std::env;
use std::fs;

#[cfg_attr(mutants, mutants::skip)]
pub(crate) fn translate_artisan_args(args: &[String]) -> Option<Vec<String>> {
    if args.len() < 2 {
        return None;
    }
    let command = &args[1];
    if command == "db:migrate"
        || command == "db:rollback"
        || command == "db:status"
        || command == "db:seed"
        || command == "studio"
    {
        let mut translated_args = vec![args[0].clone()];
        match command.as_str() {
            "db:migrate" => translated_args.push("migrate".to_string()),
            "db:rollback" => translated_args.push("migrate:rollback".to_string()),
            "db:status" => translated_args.push("status".to_string()),
            "db:seed" => translated_args.push("db:seed".to_string()),
            _ => translated_args.push(command.clone()),
        }

        // Forward any trailing arguments
        if args.len() > 2 {
            translated_args.extend_from_slice(&args[2..]);
        }
        Some(translated_args)
    } else {
        None
    }
}

/// Intercepts command line database calls (like `db:migrate` or `studio`) before AXUM web server starts.
/// Parses Rullst.toml, connects to the database, executes the requested command, and exits.
#[cfg_attr(mutants, mutants::skip)]
pub async fn check_and_run_artisan(
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn Seeder>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if let Some(translated_args) = translate_artisan_args(&args) {
        // 1. Parse database URL from Rullst.toml
        let mut db_url = None;
        if let Ok(toml_content) = fs::read_to_string("Rullst.toml") {
            for line in toml_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("url")
                    && let Some(val) = trimmed.split('=').nth(1)
                {
                    db_url = Some(val.trim().trim_matches('"').to_string());
                }
            }
        }

        let _ = dotenvy::from_filename_override(".env");
        let _ = dotenvy::dotenv();

        let url = if let Ok(env_db_url) = std::env::var("DATABASE_URL") {
            env_db_url
        } else if let Some(parsed) = db_url {
            parsed
        } else if std::path::Path::new("rullst.db").exists() {
            "sqlite://rullst.db".to_string()
        } else {
            "sqlite://db.sqlite?mode=rwc".to_string()
        };

        // 2. Initialize Orm database connection pool
        let _ = rullst_orm::Orm::init(&url).await;

        if args.len() >= 2 && args[1] == "studio" {
            start_studio_server().await;
            std::process::exit(0);
        }

        // 3. Delegate to rullst-orm Artisan CLI runner
        if let Err(e) = run_artisan_with_args(&translated_args, migrations, seeders).await {
            eprintln!("❌ Error: Executing artisan command failed: {}", e);
            std::process::exit(1);
        }

        // 4. Exit application cleanly so the Axum HTTP server does not boot
        std::process::exit(0);
    }

    Ok(())
}
