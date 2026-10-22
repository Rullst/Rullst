//! Opt-in application of pending migrations when [`Server`] starts.

use super::builder::{parse_switch as parse_named_switch, read_optional_environment_variable};
use super::{Server, ServerError};
use std::collections::HashMap;

/// Variable that turns startup migrations on or off, from the process
/// environment and then `.env`.
const AUTO_MIGRATE_VARIABLE: &str = "RULLST_AUTO_MIGRATE";

/// Whether startup migrations run: `RULLST_AUTO_MIGRATE` (the process value,
/// else the `.env` value) overrides the builder's `configured` setting; with
/// neither they are off.
fn auto_migrate_enabled(
    configured: Option<bool>,
    process: Option<String>,
    dotenv: &HashMap<String, String>,
) -> Result<bool, ServerError> {
    match process.or_else(|| dotenv.get(AUTO_MIGRATE_VARIABLE).cloned()) {
        Some(value) => parse_switch(&value),
        None => Ok(configured.unwrap_or(false)),
    }
}

fn parse_switch(value: &str) -> Result<bool, ServerError> {
    parse_named_switch(AUTO_MIGRATE_VARIABLE, value)
}

impl Server {
    /// Applies the registered migrations when startup migrations are enabled.
    #[cfg(feature = "orm")]
    #[cfg_attr(mutants, mutants::skip)]
    pub(super) async fn migrate_on_startup(
        &mut self,
        dotenv: &HashMap<String, String>,
    ) -> Result<(), ServerError> {
        let process = read_optional_environment_variable(AUTO_MIGRATE_VARIABLE)?;
        if !auto_migrate_enabled(self.auto_migrate, process, dotenv)? {
            return Ok(());
        }
        let migrations = self.migrations.take().ok_or_else(|| {
            ServerError::Migration(
                "no migrations are registered; call Server::with_migrations".to_string(),
            )
        })?;
        orm::apply(migrations).await
    }

    /// Without the ORM there is nothing to migrate: enabling it fails closed.
    #[cfg(not(feature = "orm"))]
    #[cfg_attr(mutants, mutants::skip)]
    pub(super) async fn migrate_on_startup(
        &mut self,
        dotenv: &HashMap<String, String>,
    ) -> Result<(), ServerError> {
        let process = read_optional_environment_variable(AUTO_MIGRATE_VARIABLE)?;
        if auto_migrate_enabled(self.auto_migrate, process, dotenv)? {
            return Err(ServerError::Database(
                "rullst-core was compiled without the `orm` feature".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "orm")]
mod orm {
    use super::ServerError;
    use rullst_orm::schema::Migration;
    use std::path::PathBuf;

    /// Runs the ORM's `migrate` command. It holds the runner lock of
    /// PostgreSQL and MySQL itself; an SQLite file is locked here.
    #[cfg_attr(mutants, mutants::skip)]
    pub(super) async fn apply(migrations: Vec<Box<dyn Migration>>) -> Result<(), ServerError> {
        let unavailable = |_| {
            ServerError::Migration(
                "no database is configured; set DATABASE_URL or use Server::with_db".to_string(),
            )
        };
        let pool = rullst_orm::Orm::try_pool().map_err(unavailable)?;
        let driver = rullst_orm::Orm::try_driver().map_err(unavailable)?;
        let _lock = if driver == "sqlite" {
            let file: Option<String> =
                sqlx::query_scalar("SELECT file FROM pragma_database_list WHERE name = 'main'")
                    .fetch_optional(pool)
                    .await
                    .map_err(|error| ServerError::Migration(error.to_string()))?;
            lock_sqlite_file(file).await?
        } else {
            None
        };
        let command = ["server".to_string(), "migrate".to_string()];
        rullst_orm::schema::run_artisan_with_args(&command, migrations, Vec::new())
            .await
            .map_err(|error| ServerError::Migration(error.to_string()))
    }

    /// Takes an exclusive lock on `<database>.rullst-migrate.lock`, waiting
    /// while another process holds it, and keeps it until the returned file
    /// is dropped. An in-memory database (no file) needs none. The lock file
    /// stays on disk.
    pub(super) async fn lock_sqlite_file(
        database: Option<String>,
    ) -> Result<Option<std::fs::File>, ServerError> {
        let Some(database) = database.filter(|path| !path.is_empty()) else {
            return Ok(None);
        };
        let path = PathBuf::from(format!("{database}.rullst-migrate.lock"));
        let locked = tokio::task::spawn_blocking(move || {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&path)?;
            file.lock()?;
            Ok::<_, std::io::Error>(file)
        })
        .await
        .map_err(|_| ServerError::Migration("the SQLite migration lock task failed".to_string()))?;
        locked.map(Some).map_err(|error| {
            ServerError::Migration(format!(
                "the SQLite migration lock could not be taken: {}",
                error.kind()
            ))
        })
    }
}

#[cfg(test)]
#[path = "startup_migrations_tests.rs"]
mod tests;
