//! Database selection shared by the showcase binary, `Server` and Artisan.
//!
//! The binary opens the pool itself to create and seed the schema before the
//! server starts, so it must select exactly the database that `Server` and the
//! `db:*` Artisan commands would: the process `DATABASE_URL`, then
//! `DATABASE_URL` from `./.env`, then `[database].url` from `./Rullst.toml`.

use rullst::server::{ServerError, read_optional_environment_variable};
use std::path::Path;

/// Used only when neither the environment, `.env` nor `Rullst.toml` names a
/// database. `mode=rwc` creates the SQLite file on first start.
pub const DEFAULT_DATABASE_URL: &str = "sqlite://blog.db?mode=rwc";

/// Resolves the database for the project in the working directory.
pub async fn database_url() -> Result<String, ServerError> {
    database_url_for(Path::new("."), read_optional_environment_variable).await
}

/// Resolves the database for `project_dir` with the `Server` precedence,
/// reading process variables through `environment`.
pub async fn database_url_for(
    project_dir: &Path,
    environment: impl Fn(&str) -> Result<Option<String>, ServerError>,
) -> Result<String, ServerError> {
    Ok(
        rullst::server::resolve_project_database_url(project_dir, None, environment)
            .await?
            .unwrap_or_else(|| DEFAULT_DATABASE_URL.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct ProjectDir(PathBuf);

    impl ProjectDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "rullst-blog-database-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("temporary project directory");
            Self(path)
        }

        fn write(&self, file: &str, content: &str) {
            std::fs::write(self.0.join(file), content).expect("project file");
        }
    }

    impl Drop for ProjectDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn environment(
        database_url: Option<&'static str>,
    ) -> impl Fn(&str) -> Result<Option<String>, ServerError> {
        move |name: &str| {
            Ok((name == "DATABASE_URL")
                .then_some(database_url)
                .flatten()
                .map(str::to_string))
        }
    }

    #[tokio::test]
    async fn the_server_precedence_selects_the_configured_database() {
        let project = ProjectDir::new("precedence");
        project.write(
            "Rullst.toml",
            "[database]\nurl = \"sqlite://from-toml.db\"\n",
        );
        assert_eq!(
            database_url_for(&project.0, environment(None))
                .await
                .unwrap(),
            "sqlite://from-toml.db"
        );

        project.write(".env", "DATABASE_URL=sqlite://from-dotenv.db\n");
        assert_eq!(
            database_url_for(&project.0, environment(None))
                .await
                .unwrap(),
            "sqlite://from-dotenv.db"
        );

        assert_eq!(
            database_url_for(
                &project.0,
                environment(Some("sqlite:///app/data/db.sqlite"))
            )
            .await
            .unwrap(),
            "sqlite:///app/data/db.sqlite"
        );
    }

    #[tokio::test]
    async fn an_unconfigured_project_uses_the_local_default() {
        let project = ProjectDir::new("default");
        assert_eq!(
            database_url_for(&project.0, environment(None))
                .await
                .unwrap(),
            DEFAULT_DATABASE_URL
        );
    }
}
