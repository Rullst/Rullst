use std::env;
use std::fs;
use rust_eloquent::schema::{Migration, run_artisan_with_args};
use rust_eloquent::Seeder;

/// Intercepts command line database calls (like `db:migrate`) before AXUM web server starts.
/// Parses Rullst.toml, connects to the database, executes the requested command, and exits.
pub async fn check_and_run_artisan(
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn Seeder>>
) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Ok(());
    }

    let command = &args[1];
    if command == "db:migrate" || command == "db:rollback" || command == "db:status" || command == "db:seed" {
        // 1. Parse database URL from Rullst.toml
        let mut db_url = None;
        if let Ok(toml_content) = fs::read_to_string("Rullst.toml") {
            for line in toml_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("url") {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        db_url = Some(val.trim().trim_matches('"').to_string());
                    }
                }
            }
        }

        let url = db_url.unwrap_or_else(|| "sqlite://rullst.db".to_string());
        
        // 2. Initialize Eloquent database connection pool
        rust_eloquent::Eloquent::init(&url).await?;

        // 3. Translate Rullst database arguments to rust-eloquent arguments
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

        // 4. Delegate to rust-eloquent Artisan CLI runner
        if let Err(e) = run_artisan_with_args(&translated_args, migrations, seeders).await {
            eprintln!("❌ Error: Executing artisan command failed: {}", e);
            std::process::exit(1);
        }

        // 5. Exit application cleanly so the Axum HTTP server does not boot
        std::process::exit(0);
    }

    Ok(())
}
