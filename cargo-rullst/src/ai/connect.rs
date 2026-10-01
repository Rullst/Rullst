//! `cargo rullst ai connect | disconnect | status`.

use super::AiCliError;
use super::credentials::{self, KeySource, Secret, Stored};
use super::provider::{DEFAULT_OLLAMA_HOST, DETECTION_ORDER, Provider, valid_model, valid_secret};
use super::term::{Style, TermEnv};
use clap::ArgMatches;
use dialoguer::theme::{ColorfulTheme, SimpleTheme, Theme};
use std::io::{BufRead, Read};

fn prompt_error(error: dialoguer::Error) -> AiCliError {
    AiCliError::Prompt(error.to_string())
}

/// Interactive choices, generic over the dialoguer theme (static dispatch).
fn ask_provider(theme: &impl Theme) -> Result<Provider, AiCliError> {
    let labels: Vec<&str> = DETECTION_ORDER
        .iter()
        .map(|provider| provider.label())
        .collect();
    let index = dialoguer::Select::with_theme(theme)
        .with_prompt("AI provider")
        .items(&labels)
        .default(0)
        .interact()
        .map_err(prompt_error)?;
    DETECTION_ORDER
        .get(index)
        .copied()
        .ok_or_else(|| AiCliError::Usage("no provider selected".to_string()))
}

fn ask_text(theme: &impl Theme, prompt: &str, default: &str) -> Result<String, AiCliError> {
    dialoguer::Input::<String>::with_theme(theme)
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()
        .map_err(prompt_error)
}

fn ask_secret(theme: &impl Theme, prompt: &str) -> Result<String, AiCliError> {
    dialoguer::Password::with_theme(theme)
        .with_prompt(prompt)
        .allow_empty_password(true)
        .interact()
        .map_err(prompt_error)
}

/// Reads one line (the key) from standard input; it is never echoed.
fn secret_from_stdin() -> Result<String, AiCliError> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .take(8 * 1024 + 2)
        .read_line(&mut line)?;
    Ok(line.trim_end_matches(['\n', '\r']).to_string())
}

struct Answers {
    provider: Provider,
    model: Option<String>,
    secret: Option<String>,
}

fn gather(
    matches: &ArgMatches,
    interactive: bool,
    theme: &impl Theme,
) -> Result<Answers, AiCliError> {
    let provider = match matches.get_one::<String>("provider") {
        Some(value) => Provider::parse(value)
            .ok_or_else(|| AiCliError::Usage("unknown provider".to_string()))?,
        None if interactive => ask_provider(theme)?,
        None => {
            return Err(AiCliError::Usage(
                "`cargo rullst ai connect` needs --provider when it is not run in an interactive terminal"
                    .to_string(),
            ));
        }
    };
    let model = match matches.get_one::<String>("model") {
        Some(model) => Some(model.trim().to_string()),
        None if interactive => {
            let model = ask_text(theme, "Model", provider.default_model())?;
            let model = model.trim().to_string();
            (model != provider.default_model()).then_some(model)
        }
        None => None,
    };
    if model.as_deref().is_some_and(|model| !valid_model(model)) {
        return Err(AiCliError::Usage(
            "invalid model name; use letters, digits and . - _ : / only".to_string(),
        ));
    }
    let secret = if provider.uses_api_key() {
        if matches.get_flag("api-key-stdin") {
            Some(secret_from_stdin()?)
        } else if interactive {
            let prompt = format!(
                "API key for {provider} (hidden; leave empty to use ${} instead)",
                provider.env_var()
            );
            Some(ask_secret(theme, &prompt)?).filter(|key| !key.trim().is_empty())
        } else {
            None
        }
    } else {
        match matches.get_one::<String>("host") {
            Some(host) => Some(host.trim().to_string()),
            None if interactive => Some(ask_text(theme, "Ollama host", DEFAULT_OLLAMA_HOST)?),
            None => Some(DEFAULT_OLLAMA_HOST.to_string()),
        }
    };
    if secret
        .as_deref()
        .is_some_and(|secret| !valid_secret(secret.trim()))
    {
        return Err(AiCliError::Usage(if provider.uses_api_key() {
            "the API key must be one printable token without spaces (8 KiB at most)".to_string()
        } else {
            "the Ollama host must be one token without spaces".to_string()
        }));
    }
    Ok(Answers {
        provider,
        model,
        secret: secret.map(|secret| secret.trim().to_string()),
    })
}

pub(super) fn connect(matches: &ArgMatches, env: &TermEnv, style: Style) -> Result<(), AiCliError> {
    let path = credentials::credentials_path()?;
    credentials::check_location(&path, super::project_root().as_deref(), true)?;
    let interactive = env.interactive();
    let answers = if style.color {
        gather(matches, interactive, &ColorfulTheme::default())?
    } else {
        gather(matches, interactive, &SimpleTheme)?
    };
    let Answers {
        provider,
        model,
        secret,
    } = answers;
    let stored = Stored {
        provider,
        model: model.clone(),
        secret: secret.clone().map(Secret::new),
    };
    credentials::save(&path, &stored)?;
    let model = model.unwrap_or_else(|| format!("{} (default)", provider.default_model()));
    println!(
        "{}",
        style.green(&format!("✓ Connected to {provider} · {model}"))
    );
    println!("  Saved to {} (owner-only permissions).", path.display());
    let env_set = std::env::var(provider.env_var()).is_ok_and(|value| !value.trim().is_empty());
    if provider.uses_api_key() {
        match &secret {
            None if !env_set => println!(
                "  No key stored: set {} or run `cargo rullst ai connect` again.",
                provider.env_var()
            ),
            Some(secret) if super::provider::is_mock_credential(secret) => {
                println!("  The `mock_` key selects the deterministic offline assistant.");
            }
            _ => {}
        }
    }
    if env_set {
        println!(
            "  {} is set in this environment and takes precedence over the saved value.",
            provider.env_var()
        );
    }
    println!("  Start chatting with `cargo rullst ai`.");
    Ok(())
}

pub(super) fn disconnect(style: Style) -> Result<(), AiCliError> {
    let path = credentials::credentials_path()?;
    if credentials::remove(&path)? {
        println!("{}", style.green(&format!("✓ Removed {}", path.display())));
    } else {
        println!("No stored AI credentials at {}.", path.display());
    }
    let active: Vec<&str> = DETECTION_ORDER
        .iter()
        .map(|provider| provider.env_var())
        .filter(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
        .collect();
    if !active.is_empty() {
        println!(
            "Environment variables still configure a provider: {}.",
            active.join(", ")
        );
    }
    Ok(())
}

pub(super) fn status(matches: &ArgMatches, style: Style) -> Result<(), AiCliError> {
    let path = credentials::credentials_path().ok();
    let root = super::project_root();
    let loaded = match &path {
        Some(path) => {
            credentials::check_location(path, root.as_deref(), false)?;
            credentials::load(path)?
        }
        None => None,
    };
    let resolved = credentials::resolve(
        None,
        None,
        loaded.as_ref().map(|loaded| &loaded.stored),
        |name| std::env::var(name).ok(),
    )?;
    let source = match &resolved.source {
        KeySource::Env(name) => format!("environment ({name})"),
        KeySource::File => "credentials file".to_string(),
        KeySource::Default => "default local host".to_string(),
        KeySource::Missing => "none".to_string(),
    };
    let provider = resolved.provider.map_or("none", Provider::id);
    let streaming = resolved.provider.is_some_and(Provider::streams) && !resolved.mock;
    let file = path.as_ref().map(|path| path.display().to_string());
    if matches.get_flag("json") {
        let report = serde_json::json!({
            "schema": "rullst.ai-status.v1",
            "provider": resolved.provider.map(Provider::id),
            "model": resolved.model,
            "model_is_default": resolved.model_is_default,
            "credential_source": source,
            "offline_mock": resolved.mock,
            "streaming": streaming,
            "credentials_file": file,
            "credentials_file_present": loaded.is_some(),
            "insecure_permissions": loaded.as_ref().is_some_and(|loaded| loaded.insecure_permissions),
        });
        println!("{report}");
        return Ok(());
    }
    println!("{}", style.bold("Rullst AI status"));
    println!("  provider:    {provider}");
    let default = if resolved.model_is_default {
        " (default)"
    } else {
        ""
    };
    println!("  model:       {}{default}", resolved.model);
    println!("  credential:  {source}");
    if resolved.mock {
        println!("  mode:        offline mock (no network)");
    } else {
        let delivery = if streaming {
            "streaming"
        } else {
            "complete answers"
        };
        println!("  mode:        live · {delivery}");
    }
    match (&file, &loaded) {
        (Some(file), Some(_)) => println!("  stored in:   {file}"),
        (Some(file), None) => println!("  stored in:   nothing saved ({file})"),
        (None, _) => println!("  stored in:   no user configuration directory"),
    }
    if loaded
        .as_ref()
        .is_some_and(|loaded| loaded.insecure_permissions)
    {
        println!(
            "{}",
            style.yellow(
                "  ! the credentials file is readable by other users; run `chmod 600` on it"
            )
        );
    }
    Ok(())
}
