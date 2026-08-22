use super::validation::validate_table_name;
use crate::Error;

#[async_trait::async_trait]
pub trait Migration: Send + Sync {
    fn name(&self) -> &'static str;
    async fn up(&self) -> Result<(), Error>;
    async fn down(&self) -> Result<(), Error>;
}

#[cfg_attr(test, mutants::skip)]
pub async fn run_artisan_with_args(
    args: &[String],
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn crate::Seeder>>,
) -> Result<(), Error> {
    if args.len() < 2 {
        println!("Rullst ORM Artisan CLI");
        println!("Usage:");
        println!("  make:migration <name>   Generate a new migration");
        println!("  migrate                  Run all pending migrations");
        println!("  migrate:rollback         Rollback the last batch of migrations");
        println!("  status                   Show migrations status");
        println!("  db:seed                  Populate the database with seeders");
        println!(
            "  sail:install             Generate a default docker-compose.yml (Laravel Sail style)"
        );
        return Ok(());
    }

    let command = &args[1];
    match command.as_str() {
        "make:migration" => {
            if args.len() < 3 {
                println!("Error: migration name is required.");
                return Ok(());
            }
            let name = &args[2];
            create_migration_files(name)?;
        }
        "migrate" | "db:migrate" => {
            run_migrations(migrations).await?;
        }
        "migrate:rollback" | "db:rollback" => {
            rollback_migrations(migrations).await?;
        }
        "status" | "db:status" => {
            status_migrations(migrations).await?;
        }
        "db:seed" => {
            println!("Seeding database...");
            crate::Orm::seed(seeders).await?;
            println!("Database seeded successfully!");
        }
        "sail:install" => {
            println!("Generating docker-compose.yml...");
            let content = r#"version: '3'
services:
  postgres:
    image: postgres:15
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: rullst
      POSTGRES_USER: root
      POSTGRES_PASSWORD: password
    volumes:
      - sail-postgres:/var/lib/postgresql/data
  redis:
    image: redis:alpine
    ports:
      - "6379:6379"
    volumes:
      - sail-redis:/data
  meilisearch:
    image: getmeili/meilisearch:latest
    ports:
      - "7700:7700"
    environment:
      MEILI_MASTER_KEY: sail
    volumes:
      - sail-meilisearch:/meili_data
  pgadmin:
    image: dpage/pgadmin4
    ports:
      - "5050:80"
    environment:
      PGADMIN_DEFAULT_EMAIL: admin@rullst.com
      PGADMIN_DEFAULT_PASSWORD: password

volumes:
  sail-postgres:
    driver: local
  sail-redis:
    driver: local
  sail-meilisearch:
    driver: local
"#;
            std::fs::write("docker-compose.yml", content).map_err(|e| {
                crate::Error::Internal(format!("Failed to write docker-compose.yml: {}", e))
            })?;
            println!(
                "docker-compose.yml created successfully! Run `docker compose up -d` to start."
            );
        }
        _ => {
            println!("Unknown command: {}", command);
        }
    }
    Ok(())
}

#[cfg_attr(test, mutants::skip)]
pub async fn run_artisan(
    migrations: Vec<Box<dyn Migration>>,
    seeders: Vec<Box<dyn crate::Seeder>>,
) -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    run_artisan_with_args(&args, migrations, seeders).await
}

#[mutants::skip]
async fn migrations_table_exists(pool: &crate::RullstPool, driver: &str) -> Result<bool, Error> {
    match driver {
        "postgres" | "mysql" => {
            let query_str =
                "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await?;
            Ok(row.0 > 0)
        }
        _ => {
            let query_str =
                "SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='migrations'";
            let row: (i64,) = sqlx::query_as(query_str).fetch_one(pool).await?;
            Ok(row.0 > 0)
        }
    }
}

#[cfg_attr(test, mutants::skip)]
async fn status_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Orm::try_pool()?;
    let driver = crate::Orm::try_driver()?;

    let table_exists = migrations_table_exists(pool, driver).await?;

    let executed_set = if table_exists {
        let executed: Vec<(String,)> = sqlx::query_as("SELECT migration FROM migrations")
            .fetch_all(pool)
            .await?;
        executed
            .into_iter()
            .map(|(m,)| m)
            .collect::<std::collections::HashSet<String>>()
    } else {
        std::collections::HashSet::new()
    };

    let name_header = "Migration Name";
    let status_header = "Status";
    println!("{name_header:<40} | {status_header}");
    println!("{}", "-".repeat(55));
    for m in migrations {
        let name = m.name();
        let status = if executed_set.contains(name) {
            "Applied"
        } else {
            "Pending"
        };
        println!("{:<40} | {}", name, status);
    }

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
fn create_migration_files(name: &str) -> Result<(), Error> {
    validate_table_name(name)?;
    use std::fs;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let sanitized_name = name.replace(['/', '\\'], "");
    let snake_name = sanitized_name.to_lowercase().replace("-", "_");
    let file_name = format!("m{}_{}", now, snake_name);

    fs::create_dir_all("src/migrations")
        .map_err(|e| Error::Internal(format!("Failed to create migrations directory: {}", e)))?;

    let new_file_path = format!("src/migrations/{}.rs", file_name);
    let template = include_str!("../migration_template.rs.txt");
    let migration_code = template
        .replace("{timestamp}", &now)
        .replace("{name}", &snake_name);

    fs::write(&new_file_path, migration_code)
        .map_err(|e| Error::Internal(format!("Failed to write migration file: {}", e)))?;
    println!("Created migration file: {}", new_file_path);

    regenerate_migrations_mod()?;

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
fn regenerate_migrations_mod() -> Result<(), Error> {
    use std::fs;
    let paths = fs::read_dir("src/migrations")
        .map_err(|e| Error::Internal(format!("Failed to read migrations dir: {}", e)))?;

    let mut modules = vec![];
    for path in paths {
        let path = path.map_err(|e| Error::Internal(e.to_string()))?.path();
        if let Some(ext) = path.extension()
            && ext == "rs"
            && let Some(stem) = path.file_stem()
        {
            let stem_str = stem.to_string_lossy().to_string();
            if stem_str != "mod" && stem_str.starts_with('m') {
                modules.push(stem_str);
            }
        }
    }
    modules.sort();

    use std::fmt::Write;
    let mut mod_content = String::new();
    mod_content.push_str("// Generated by Rullst ORM Artisan. Do not edit manually.\n\n");
    for m in &modules {
        let _ = writeln!(mod_content, "pub mod {};", m);
    }
    mod_content
        .push_str("\npub fn get_migrations() -> Vec<Box<dyn rullst_orm::schema::Migration>> {\n");
    mod_content.push_str("    vec![\n");
    for m in &modules {
        let _ = writeln!(mod_content, "        Box::new({}::MigrationImpl),", m);
    }
    mod_content.push_str("    ]\n");
    mod_content.push_str("}\n");

    fs::write("src/migrations/mod.rs", mod_content)
        .map_err(|e| Error::Internal(format!("Failed to write mod.rs: {}", e)))?;
    println!("Regenerated src/migrations/mod.rs");

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
async fn run_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Orm::try_pool()?;
    let driver = crate::Orm::try_driver()?;

    let query_str = match driver {
        "postgres" => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id SERIAL PRIMARY KEY,
                migration VARCHAR(255) NOT NULL,
                batch INTEGER NOT NULL
            )"
        }
        "mysql" => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id INT AUTO_INCREMENT PRIMARY KEY,
                migration VARCHAR(255) NOT NULL,
                batch INT NOT NULL
            )"
        }
        _ => {
            "CREATE TABLE IF NOT EXISTS migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                migration TEXT NOT NULL,
                batch INTEGER NOT NULL
            )"
        }
    };

    sqlx::query(query_str).execute(pool).await?;

    let executed: Vec<(String,)> = sqlx::query_as("SELECT migration FROM migrations")
        .fetch_all(pool)
        .await?;
    let executed_set: std::collections::HashSet<String> =
        executed.into_iter().map(|(m,)| m).collect();

    let batch_row: (Option<i32>,) = sqlx::query_as("SELECT MAX(batch) FROM migrations")
        .fetch_one(pool)
        .await?;
    let next_batch = batch_row.0.unwrap_or(0) + 1;

    let mut count = 0;
    let mut successful_migrations = vec![];
    for m in migrations {
        let name = m.name();
        if !executed_set.contains(name) {
            println!("Migrating: {}", name);
            m.up().await?;
            successful_migrations.push(name);
            println!("Migrated:  {}", name);
            count += 1;
        }
    }

    if count > 0 {
        let mut query_builder =
            sqlx::query_builder::QueryBuilder::new("INSERT INTO migrations (migration, batch) ");
        query_builder.push_values(successful_migrations, |mut b, name| {
            b.push_bind(name).push_bind(next_batch);
        });
        query_builder.build().execute(pool).await?;
    } else {
        println!("Nothing to migrate.");
    }

    Ok(())
}

#[cfg_attr(test, mutants::skip)]
async fn rollback_migrations(migrations: Vec<Box<dyn Migration>>) -> Result<(), Error> {
    let pool = crate::Orm::try_pool()?;
    let driver = crate::Orm::try_driver()?;

    let table_exists = migrations_table_exists(pool, driver).await?;

    if !table_exists {
        println!("Nothing to rollback.");
        return Ok(());
    }

    let batch_row: (Option<i32>,) = sqlx::query_as("SELECT MAX(batch) FROM migrations")
        .fetch_one(pool)
        .await?;

    let last_batch = match batch_row.0 {
        Some(b) if b > 0 => b,
        _ => {
            println!("Nothing to rollback.");
            return Ok(());
        }
    };

    let to_rollback: Vec<(String,)> =
        sqlx::query_as("SELECT migration FROM migrations WHERE batch = ? ORDER BY id DESC")
            .bind(last_batch)
            .fetch_all(pool)
            .await?;

    let mut rollback_map = std::collections::HashMap::with_capacity(migrations.len());
    for m in migrations {
        rollback_map.insert(m.name().to_string(), m);
    }

    let mut rolled_back = Vec::with_capacity(to_rollback.len());
    for (name,) in to_rollback {
        if let Some(m) = rollback_map.get(&name) {
            println!("Rolling back: {}", name);
            m.down().await?;
            println!("Rolled back:  {}", name);
            rolled_back.push(name);
        } else {
            println!(
                "Warning: migration {} found in database but not in compiled binary.",
                name
            );
        }
    }

    if !rolled_back.is_empty() {
        let mut query_builder =
            sqlx::query_builder::QueryBuilder::new("DELETE FROM migrations WHERE migration IN (");
        let mut separated = query_builder.separated(", ");
        for name in rolled_back {
            separated.push_bind(name);
        }
        separated.push_unseparated(")");
        query_builder.build().execute(pool).await?;
    }

    Ok(())
}
