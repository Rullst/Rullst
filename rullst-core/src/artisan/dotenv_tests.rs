//! Artisan commands and the legacy Studio read `.env` without loading it into
//! the process environment, like `Server`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::artisan::runner::{ArtisanError, run_artisan_command, translate_artisan_args};
use crate::artisan::studio_views::is_ai_configured_in;

const AI_KEYS: [&str; 4] = [
    "GEMINI_API_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "DEEPSEEK_API_KEY",
];

/// Removes `keys` from the process environment and restores them when dropped.
struct Unset(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl Unset {
    fn new(keys: &[&'static str]) -> Self {
        let saved = keys
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect();
        // SAFETY: callers hold `TEST_ENV_LOCK`, which serializes this crate's
        // environment changes.
        for key in keys {
            unsafe { std::env::remove_var(key) };
        }
        Self(saved)
    }
}

impl Drop for Unset {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn project(dotenv: &str) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join(".env"), dotenv).unwrap();
    directory
}

#[tokio::test]
async fn studio_reads_ai_keys_from_dotenv_without_exporting_them() {
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let _unset = Unset::new(&AI_KEYS);

    let configured = project("OPENAI_API_KEY=fixture-value\n");
    assert!(is_ai_configured_in(configured.path()));
    assert_eq!(std::env::var_os("OPENAI_API_KEY"), None);

    for dotenv in [
        "OPENAI_API_KEY=   \n",
        "OTHER=1\n",
        "OPENAI_API_KEY='unterminated\n",
    ] {
        assert!(!is_ai_configured_in(project(dotenv).path()), "{dotenv}");
    }
    let without_dotenv = tempfile::tempdir().unwrap();
    assert!(!is_ai_configured_in(without_dotenv.path()));
}

#[tokio::test]
async fn artisan_commands_never_load_dotenv_into_the_process() {
    const PROBE: &str = "RULLST_ARTISAN_DOTENV_EXPORT_PROBE";
    let _lock = crate::server::TEST_ENV_LOCK.lock().await;
    let _unset = Unset::new(&[PROBE]);
    // Holding the Studio port makes `studio` return right after the point
    // where the runner used to export `.env`.
    let Ok(_studio_port) = tokio::net::TcpListener::bind("127.0.0.1:5555").await else {
        eprintln!("skipped: 127.0.0.1:5555 is in use by another process");
        return;
    };
    let project = project(&format!("{PROBE}=from-dotenv\n"));
    let args = translate_artisan_args(&["app".to_string(), "studio".to_string()]).unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        run_artisan_command("studio", &args, None, None, project.path(), |_| Ok(None)),
    )
    .await
    .expect("studio fails to bind instead of serving");
    assert!(matches!(result, Err(ArtisanError::Studio(_))), "{result:?}");
    assert_eq!(std::env::var_os(PROBE), None);
}
