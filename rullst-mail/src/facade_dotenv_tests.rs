//! `.env` resolution of the facade settings, in hermetic project directories.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::settings::MailSettings;
use super::tests::{EnvironmentGuard, clear_provider_environment};
use super::*;
use std::path::{Path, PathBuf};

/// A temporary project directory with a `.env` and an optional `Rullst.toml`.
struct Project(PathBuf);

impl Project {
    fn new(dotenv: &str, rullst_toml: Option<&str>) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rullst-mail-dotenv-{}-{}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join(".env"), dotenv).unwrap();
        if let Some(content) = rullst_toml {
            std::fs::write(root.join("Rullst.toml"), content).unwrap();
        }
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn isolated_environment() -> EnvironmentGuard {
    let mut environment = EnvironmentGuard::new();
    clear_provider_environment(&mut environment);
    for key in ["MAIL_DRIVER", "MAIL_FROM", "RULLST_ENV", "APP_ENV"] {
        environment.clear(key);
    }
    environment
}

#[tokio::test]
async fn dotenv_settings_apply_after_the_process_environment_and_before_rullst_toml() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = isolated_environment();
    let project = Project::new(
        "MAIL_DRIVER=memory\nMAIL_FROM=\"Acme <billing@acme.example>\"\n",
        Some("[mail]\ndriver = \"log\"\nfrom = \"toml@acme.example\"\n"),
    );

    let settings = MailSettings::load_from(project.path()).await.unwrap();
    assert_eq!(settings.driver_name().unwrap(), "memory");
    assert_eq!(
        settings.default_sender().unwrap(),
        Some("Acme <billing@acme.example>")
    );

    environment.set("MAIL_DRIVER", "log");
    environment.set("MAIL_FROM", "ops@acme.example");
    let settings = MailSettings::load_from(project.path()).await.unwrap();
    assert_eq!(settings.driver_name().unwrap(), "log");
    assert_eq!(settings.default_sender().unwrap(), Some("ops@acme.example"));

    let toml_only = Project::new("", Some("[mail]\nfrom = \"toml@acme.example\"\n"));
    environment.clear("MAIL_FROM");
    let settings = MailSettings::load_from(toml_only.path()).await.unwrap();
    assert_eq!(
        settings.default_sender().unwrap(),
        Some("toml@acme.example")
    );
}

#[tokio::test]
async fn dotenv_provider_credentials_select_the_real_transport() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = isolated_environment();
    let project = Project::new(
        "MAIL_DRIVER=resend\nRESEND_API_KEY=re_dotenv_fixture\n",
        None,
    );
    let message = Message::new()
        .to("member@example.com")
        .subject("Credential source")
        .text("body");

    // The real transport refuses a message without a sender before any
    // request; the offline mock would have accepted it.
    let settings = MailSettings::load_from(project.path()).await.unwrap();
    let driver = Mail::resolve_driver_from(&settings).unwrap();
    assert!(matches!(
        driver.send(&message).await,
        Err(MailError::ConfigError(_))
    ));

    // The process environment wins, even with an empty (offline) credential.
    environment.set("RESEND_API_KEY", "");
    let settings = MailSettings::load_from(project.path()).await.unwrap();
    let driver = Mail::resolve_driver_from(&settings).unwrap();
    driver.send(&message).await.unwrap();
}

#[tokio::test]
async fn the_environment_is_detected_from_dotenv_like_server() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let mut environment = isolated_environment();
    let production = Project::new("RULLST_ENV=production\n", None);
    let settings = MailSettings::load_from(production.path()).await.unwrap();
    assert!(matches!(
        settings.driver_name(),
        Err(MailError::ConfigError(text)) if text.contains("production")
    ));

    let staging = Project::new("APP_ENV=staging\n", Some("[app]\nenv = \"development\"\n"));
    let settings = MailSettings::load_from(staging.path()).await.unwrap();
    assert!(matches!(
        settings.driver_name(),
        Err(MailError::ConfigError(_))
    ));

    let development = Project::new(
        "RULLST_ENV=development\n",
        Some("[app]\nenv = \"production\"\n"),
    );
    let settings = MailSettings::load_from(development.path()).await.unwrap();
    assert_eq!(settings.driver_name().unwrap(), "log");

    environment.set("RULLST_ENV", "development");
    let settings = MailSettings::load_from(production.path()).await.unwrap();
    assert_eq!(settings.driver_name().unwrap(), "log");
}

#[tokio::test]
async fn malformed_dotenv_fails_closed_without_echoing_it() {
    let _lock = MAIL_ENV_LOCK.lock().await;
    let _environment = isolated_environment();
    let project = Project::new("MAIL_FROM='unterminated-secret-marker\n", None);
    let error = MailSettings::load_from(project.path()).await.unwrap_err();
    assert!(matches!(&error, MailError::ConfigError(_)));
    let message = error.to_string();
    assert!(message.contains(".env"));
    assert!(!message.contains("secret-marker"));
}
