use super::*;
use std::path::PathBuf;

/// A throwaway project directory removed on drop.
struct TempProject(PathBuf);

impl TempProject {
    fn new(label: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "rullst-studio-{label}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("temporary Studio project");
        Self(path)
    }

    fn write(&self, name: &str, content: &str) {
        std::fs::write(self.0.join(name), content).expect("temporary project file");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn environment(
    value: Option<&'static str>,
) -> impl Fn(&str) -> Result<Option<String>, ServerError> {
    move |name| {
        Ok((name == "DATABASE_URL")
            .then_some(value)
            .flatten()
            .map(str::to_string))
    }
}

#[tokio::test]
// Studio must select the same database as Server and Artisan and must not
// fall back to creating `db.sqlite`.
async fn studio_resolution_matches_the_shared_server_precedence() {
    let empty = TempProject::new("empty");
    assert_eq!(
        resolve_studio_database_url(&empty.0, environment(None))
            .await
            .expect("empty project resolves"),
        None
    );
    assert!(!empty.0.join("db.sqlite").exists());

    let layered = TempProject::new("layered");
    layered.write(".env", "DATABASE_URL=sqlite://from-dotenv.db\n");
    layered.write(
        "Rullst.toml",
        "[database]\nurl = \"sqlite://from-toml.db\"\n",
    );
    assert_eq!(
        resolve_studio_database_url(&layered.0, environment(None))
            .await
            .expect("dotenv project resolves")
            .as_deref(),
        Some("sqlite://from-dotenv.db")
    );
    assert_eq!(
        resolve_studio_database_url(&layered.0, environment(Some("sqlite://from-process.db")))
            .await
            .expect("process environment resolves")
            .as_deref(),
        Some("sqlite://from-process.db")
    );

    let toml_only = TempProject::new("toml");
    toml_only.write(
        "Rullst.toml",
        "[redis]\nurl = \"redis://cache.internal\"\n\n[database]\n\
         url = \"postgres://db.internal/app?application_name=studio&sslmode=verify-full\"\n",
    );
    assert_eq!(
        resolve_studio_database_url(&toml_only.0, environment(None))
            .await
            .expect("TOML project resolves")
            .as_deref(),
        Some("postgres://db.internal/app?application_name=studio&sslmode=verify-full")
    );

    let other_section = TempProject::new("section");
    other_section.write("Rullst.toml", "[redis]\nurl = \"redis://cache.internal\"\n");
    assert_eq!(
        resolve_studio_database_url(&other_section.0, environment(None))
            .await
            .expect("project without a database section resolves"),
        None
    );
}

#[tokio::test]
async fn studio_resolution_errors_do_not_echo_configuration_content() {
    let malformed = TempProject::new("malformed");
    malformed.write(
        "Rullst.toml",
        "[database]\nurl = \"postgres://studio-marker-7f3a@db.internal/app\n",
    );
    let error = resolve_studio_database_url(&malformed.0, environment(None))
        .await
        .expect_err("unterminated TOML string is rejected")
        .to_string();
    assert!(!error.contains("studio-marker-7f3a"));
    assert!(!error.contains("db.internal"));
}
