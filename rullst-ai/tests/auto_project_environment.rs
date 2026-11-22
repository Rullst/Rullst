//! `AiClient::auto()` resolves staging and production as `Server` does: the
//! process `RULLST_ENV`, then `APP_ENV`, then either one from `.env`, then
//! `[app] env` in `Rullst.toml`. The working directory and the process
//! environment are global, so this binary holds a single test.

use rullst_ai::{AiClient, AiError, AutoAiConfig};
use std::fs;
use std::path::{Path, PathBuf};

const VARIABLES: [&str; 9] = [
    "RULLST_ENV",
    "APP_ENV",
    "OPENAI_API_KEY",
    "OPENAI_BASE_URL",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "DEEPSEEK_API_KEY",
    "GROQ_API_KEY",
    "OLLAMA_HOST",
];

fn set(name: &str, value: &str) {
    // SAFETY: this binary runs one test, so nothing reads the environment
    // concurrently.
    unsafe { std::env::set_var(name, value) };
}

fn remove(name: &str) {
    // SAFETY: as in `set`.
    unsafe { std::env::remove_var(name) };
}

/// Writes `.env` and `Rullst.toml` in `project`, removing the ones given as `None`.
fn files(project: &Path, dotenv: Option<&str>, config: Option<&str>) {
    for (name, content) in [(".env", dotenv), ("Rullst.toml", config)] {
        let path = project.join(name);
        match content {
            Some(content) => fs::write(&path, content).expect("write project file"),
            None if path.exists() => fs::remove_file(&path).expect("remove project file"),
            None => {}
        }
    }
}

/// The reason `AiClient::auto()` refuses, or `None` when it builds a client.
fn refusal() -> Option<String> {
    match AiClient::auto() {
        Ok(_) => None,
        Err(AiError::ConfigError(reason)) => Some(reason),
        Err(other) => panic!("unexpected error: {other}"),
    }
}

fn refuses_in(environment: &str) {
    let reason = refusal().unwrap_or_else(|| panic!("{environment}: the offline fixture was used"));
    assert!(
        reason.contains(&format!("in the {environment} environment"))
            && reason.contains("OPENAI_API_KEY"),
        "{environment}: {reason}"
    );
}

#[test]
fn staging_and_production_come_from_the_process_dotenv_and_rullst_toml() {
    let previous = std::env::current_dir().expect("current directory");
    let saved: Vec<_> = VARIABLES
        .iter()
        .map(|name| (*name, std::env::var_os(name)))
        .collect();
    for name in VARIABLES {
        remove(name);
    }
    let project: PathBuf = std::env::temp_dir().join(format!(
        "rullst-ai-auto-environment-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default()
    ));
    fs::create_dir_all(&project).expect("create project directory");
    std::env::set_current_dir(&project).expect("enter project directory");

    // Neither file: development keeps the offline fixture.
    assert_eq!(refusal(), None);
    // Only `.env` selects the environment, with either name.
    files(
        &project,
        Some("RULLST_ENV=production\nOPENAI_API_KEY=\n"),
        None,
    );
    refuses_in("production");
    files(&project, Some("APP_ENV=staging\n"), None);
    refuses_in("staging");
    // Only `[app] env` selects it.
    files(&project, None, Some("[app]\nenv = \"production\"\n"));
    refuses_in("production");
    // `.env` comes before `[app] env`, and the process before `.env`.
    files(
        &project,
        Some("RULLST_ENV=development\n"),
        Some("[app]\nenv = \"production\"\n"),
    );
    assert_eq!(refusal(), None);
    files(&project, Some("RULLST_ENV=production\n"), None);
    set("APP_ENV", "development");
    assert_eq!(refusal(), None);
    set("RULLST_ENV", "test");
    assert_eq!(refusal(), None);
    remove("RULLST_ENV");
    remove("APP_ENV");
    // A mock behind a live provider is refused when only `.env` says production.
    set("OPENAI_API_KEY", "sk-fixture-live");
    set("GEMINI_API_KEY", "mock_gemini");
    let mixed = AutoAiConfig::from_env().map(|config| config.provider_names().to_vec());
    assert!(
        matches!(&mixed, Err(AiError::ConfigError(reason))
            if reason.contains("Google Gemini (offline mock)") && reason.contains("production")),
        "{mixed:?}"
    );
    files(&project, None, None);
    assert_eq!(
        AutoAiConfig::from_env().unwrap().provider_names(),
        ["OpenAI", "Google Gemini (offline mock)"]
    );
    remove("OPENAI_API_KEY");
    remove("GEMINI_API_KEY");
    // An invalid or unreadable setting fails closed, without file content.
    files(&project, None, Some("[app]\nenv = \"nowhere\"\n"));
    assert_eq!(
        refusal().as_deref(),
        Some("RULLST_ENV, APP_ENV or [app] env is not a valid Rullst environment")
    );
    files(&project, None, Some("[app]\nenv = \"production-secret\n"));
    let reason = refusal().expect("a malformed Rullst.toml is refused");
    assert!(
        reason.starts_with("the AI environment could not be read")
            && !reason.contains("production-secret"),
        "{reason}"
    );
    files(&project, Some("RULLST_ENV=\"unterminated-secret\n"), None);
    let reason = refusal().expect("a malformed .env is refused");
    assert!(
        reason.starts_with("the AI environment could not be read")
            && !reason.contains("unterminated-secret"),
        "{reason}"
    );
    files(&project, None, None);
    fs::create_dir(project.join("Rullst.toml")).expect("unreadable Rullst.toml");
    let reason = refusal().expect("an unreadable Rullst.toml is refused");
    assert!(
        reason.starts_with("Rullst.toml could not be read: "),
        "{reason}"
    );

    std::env::set_current_dir(previous).expect("leave project directory");
    let _ = fs::remove_dir_all(&project);
    for (name, value) in saved {
        match value {
            Some(value) => set(name, &value.to_string_lossy()),
            None => remove(name),
        }
    }
}
