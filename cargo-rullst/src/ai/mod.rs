//! `cargo rullst ai`: a terminal assistant that knows Rullst and the current
//! project and can propose reviewed changes.
//!
//! - `cargo rullst ai connect|disconnect|status` manage user-level provider
//!   settings outside every project ([`credentials`]).
//! - `cargo rullst ai` opens a streaming chat; `cargo rullst ai "<goal>"` runs
//!   one goal. Answers go through `rullst-ai` and its guardrails
//!   ([`backend`]); without a configured key a deterministic offline
//!   assistant answers ([`mock`]).
//! - The model may only propose the actions of [`protocol`]: allowlisted
//!   `cargo rullst` commands, file writes below the project root and
//!   `cargo check`/`cargo test`. Each is previewed and confirmed; nothing runs
//!   without an interactive terminal. A git checkpoint precedes the first
//!   change of a session ([`checkpoint`]).
//! - `cargo rullst ai upgrade` grounds a session in the findings of the
//!   framework upgrade plan ([`upgrade`]).
//! - `cargo rullst ai fix <error-id>` grounds a session in a panic recorded
//!   by the development error console ([`fix`]).
//! - `cargo rullst ai review` reviews the current diff read-only
//!   ([`review`]): no actions, secrets dropped or redacted first.

use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

mod actions;
mod backend;
mod checkpoint;
mod coalesce;
mod commands;
mod connect;
mod credentials;
pub(crate) mod diff;
mod environment;
mod fix;
mod input;
mod masked;
mod mock;
mod paths;
mod process;
mod prompt;
mod protocol;
mod provider;
mod review;
mod session;
pub(crate) mod term;
mod upgrade;
mod usage;

use session::{Mode, Session, Settings};
use term::{Style, TermEnv, sanitize};

#[derive(thiserror::Error)]
pub(crate) enum AiCliError {
    #[error(transparent)]
    Credentials(#[from] credentials::CredentialError),
    #[error(transparent)]
    Model(#[from] credentials::InvalidModel),
    #[error("{0}")]
    Usage(String),
    #[error("prompt failed: {0}")]
    Prompt(String),
    #[error("AI provider configuration failed: {0}")]
    Provider(String),
    #[error("the upgrade plan failed: {0}")]
    Upgrade(String),
    #[error("{0}")]
    Fix(String),
    #[error("{0}")]
    Review(String),
    #[error("terminal I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

// `main` prints errors with Display; keep Debug identical and secret-free.
impl std::fmt::Debug for AiCliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

const PROVIDERS: [&str; 7] = [
    "openai",
    "anthropic",
    "claude",
    "gemini",
    "deepseek",
    "ollama",
    "local",
];

fn provider_arg() -> Arg {
    Arg::new("provider")
        .long("provider")
        .value_name("PROVIDER")
        .value_parser(PROVIDERS)
        .help("openai, anthropic (claude), gemini, deepseek, ollama or local (OpenAI-compatible server)")
}

fn model_arg() -> Arg {
    Arg::new("model")
        .long("model")
        .value_name("MODEL")
        .help("Model name; defaults to RULLST_AI_MODEL, the saved model or the provider default")
}

pub(crate) fn command() -> Command {
    Command::new("ai")
        .about(
            "Chat with an assistant that knows Rullst and your project, and apply reviewed changes",
        )
        .args_conflicts_with_subcommands(true)
        .arg(
            Arg::new("goal")
                .value_name("GOAL")
                .num_args(1..)
                .trailing_var_arg(true)
                .help(
                    "Run one goal and exit (quote it); without a goal, start an interactive chat",
                ),
        )
        .arg(provider_arg())
        .arg(model_arg())
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Show proposed actions without executing them"),
        )
        .subcommand(
            Command::new("connect")
                .about("Choose a provider and model and save them (and the key) for your user")
                .arg(provider_arg())
                .arg(model_arg())
                .arg(
                    Arg::new("api-key-stdin")
                        .long("api-key-stdin")
                        .action(ArgAction::SetTrue)
                        .conflicts_with_all(["host", "base-url"])
                        .help("Read the API key from the first line of standard input"),
                )
                .arg(
                    Arg::new("host")
                        .long("host")
                        .value_name("URL")
                        .conflicts_with("base-url")
                        .help("Ollama host (default http://127.0.0.1:11434)"),
                )
                .arg(
                    Arg::new("base-url")
                        .long("base-url")
                        .value_name("URL")
                        .help("Local OpenAI-compatible server on a loopback IP (default http://127.0.0.1:1234/v1)"),
                )
                .arg(
                    Arg::new("input-price-per-mtok")
                        .long("input-price-per-mtok")
                        .value_name("PRICE")
                        .value_parser(clap::value_parser!(f64))
                        .requires("output-price-per-mtok")
                        .help("Your price per million input tokens, used only for a labelled cost estimate"),
                )
                .arg(
                    Arg::new("output-price-per-mtok")
                        .long("output-price-per-mtok")
                        .value_name("PRICE")
                        .value_parser(clap::value_parser!(f64))
                        .requires("input-price-per-mtok")
                        .help("Your price per million output tokens, used only for a labelled cost estimate"),
                ),
        )
        .subcommand(
            Command::new("upgrade")
                .about("Plan the framework upgrade and review assistant-proposed fixes for its source findings")
                .arg(
                    Arg::new("to")
                        .long("to")
                        .value_name("VERSION")
                        .help("Exact target version; defaults to the installed cargo-rullst version"),
                )
                .arg(provider_arg())
                .arg(model_arg())
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .action(ArgAction::SetTrue)
                        .help("Show proposed fixes without executing them"),
                ),
        )
        .subcommand(fix::command(provider_arg(), model_arg()))
        .subcommand(review::command(provider_arg(), model_arg()))
        .subcommand(Command::new("disconnect").about("Delete the saved AI credentials file"))
        .subcommand(
            Command::new("status")
                .about("Show the provider, model and credential source in use (never the key)")
                .arg(
                    Arg::new("json")
                        .long("json")
                        .action(ArgAction::SetTrue)
                        .help("Emit the rullst.ai-status.v1 JSON report"),
                ),
        )
}

/// The nearest directory at or above the current one with a `Cargo.toml`.
fn project_root() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    current
        .ancestors()
        .find(|directory| {
            std::fs::symlink_metadata(directory.join("Cargo.toml"))
                .is_ok_and(|metadata| metadata.is_file())
        })
        .and_then(|directory| std::fs::canonicalize(directory).ok())
}

/// The handler a parsed `cargo rullst ai` invocation selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    Connect,
    Disconnect,
    Status,
    Upgrade,
    Fix,
    Review,
    Session,
}

/// Selects the handler and the matches it reads (the subcommand's own, or
/// the top-level ones for a chat session).
fn route(matches: &ArgMatches) -> (Route, &ArgMatches) {
    match matches.subcommand() {
        Some(("connect", connect)) => (Route::Connect, connect),
        Some(("disconnect", disconnect)) => (Route::Disconnect, disconnect),
        Some(("status", status)) => (Route::Status, status),
        Some(("upgrade", upgrade)) => (Route::Upgrade, upgrade),
        Some(("fix", fix)) => (Route::Fix, fix),
        Some(("review", review)) => (Route::Review, review),
        _ => (Route::Session, matches),
    }
}

pub(crate) fn run(matches: &ArgMatches) -> Result<(), AiCliError> {
    let env = TermEnv::detect();
    let style = Style { color: env.color() };
    let (route, selected) = route(matches);
    match route {
        Route::Connect => connect::connect(selected, &env, style),
        Route::Disconnect => connect::disconnect(style),
        Route::Status => connect::status(selected, style),
        Route::Upgrade => run_upgrade(selected, &env, style),
        Route::Fix => run_fix(selected, &env, style),
        Route::Review => review::run(selected, style),
        Route::Session => run_session(selected, &env, style, None),
    }
}

/// Plans the upgrade; a plan with findings continues as an assistant session.
fn run_upgrade(matches: &ArgMatches, env: &TermEnv, style: Style) -> Result<(), AiCliError> {
    let root = project_root().ok_or_else(|| {
        AiCliError::Usage("run `cargo rullst ai upgrade` inside a Rullst project".to_string())
    })?;
    let plan = crate::generators::build::assist_plan(
        &root,
        matches.get_one::<String>("to").map(String::as_str),
    )
    .map_err(|error| AiCliError::Upgrade(sanitize(&error.to_string())))?;
    if plan.findings.is_empty() {
        println!("{}", sanitize(&plan.summary));
        println!(
            "{}",
            style.green(no_findings_next_step(plan.pending_changes))
        );
        return Ok(());
    }
    run_session(matches, env, style, Some(Grounding::Upgrade(root, plan)))
}

/// Whether a session reads standard input. A one-shot plan never prompts, so
/// standard input is left untouched.
fn reads_stdin(has_goal: bool, mode: &Mode) -> bool {
    !(has_goal && *mode != Mode::Execute)
}

/// The next step printed when the upgrade plan has no source findings.
fn no_findings_next_step(pending_changes: usize) -> &'static str {
    if pending_changes > 0 {
        "No source findings need the assistant. Apply the dependency plan with `cargo rullst upgrade`."
    } else {
        "No source findings need the assistant and the dependencies already target this release."
    }
}

/// A session grounded in prepared data: the upgrade plan or a recorded error.
enum Grounding {
    Upgrade(PathBuf, crate::generators::build::AssistPlan),
    Fix(fix::Prepared),
}

/// Fetches the recorded error and continues as an assistant session.
fn run_fix(matches: &ArgMatches, env: &TermEnv, style: Style) -> Result<(), AiCliError> {
    let root = project_root().ok_or_else(|| {
        AiCliError::Usage("run `cargo rullst ai fix` inside a Rullst project".to_string())
    })?;
    let prepared = fix::prepare(matches, &root)?;
    run_session(matches, env, style, Some(Grounding::Fix(prepared)))
}

/// The guarded backend for the configured provider (or the offline
/// assistant), its description and the user's prices.
fn connect_backend(
    matches: &ArgMatches,
    root: Option<&std::path::Path>,
    style: Style,
) -> Result<
    (
        backend::Backend,
        backend::Description,
        Option<credentials::Prices>,
    ),
    AiCliError,
> {
    let path = credentials::credentials_path().ok();
    let loaded = match &path {
        Some(path) => {
            credentials::check_location(path, root, false)?;
            credentials::load(path)?
        }
        None => None,
    };
    if loaded
        .as_ref()
        .is_some_and(|loaded| loaded.insecure_permissions)
    {
        eprintln!(
            "{}",
            style.yellow(
                "warning: the AI credentials file is readable by other users; run `chmod 600` on it"
            )
        );
    }
    let flag_provider = matches
        .get_one::<String>("provider")
        .and_then(|value| provider::Provider::parse(value));
    let resolved = credentials::resolve(
        flag_provider,
        matches.get_one::<String>("model").map(String::as_str),
        loaded.as_ref().map(|loaded| &loaded.stored),
        |name| std::env::var(name).ok(),
    )?;
    let (backend, description) = backend::Backend::build(&resolved)
        .map_err(|error| AiCliError::Provider(sanitize(&error.to_string())))?;
    Ok((backend, description, resolved.prices))
}

fn run_session(
    matches: &ArgMatches,
    env: &TermEnv,
    style: Style,
    grounding: Option<Grounding>,
) -> Result<(), AiCliError> {
    let root = project_root();
    let (backend, description, prices) = connect_backend(matches, root.as_deref(), style)?;
    let notice = if description.offline {
        Some(
            "No provider is connected: a deterministic offline assistant answers. Run `cargo rullst ai connect` to use a model."
                .to_string(),
        )
    } else if description.streaming {
        None
    } else {
        Some(
            "This Ollama host is not a loopback address, so answers arrive at once instead of streaming."
                .to_string(),
        )
    };
    let mode = if matches.get_flag("dry-run") {
        Mode::PlanOnly("--dry-run")
    } else if env.interactive() {
        Mode::Execute
    } else {
        Mode::PlanOnly("not an interactive terminal")
    };
    let goal = match &grounding {
        Some(Grounding::Upgrade(..)) => Some(upgrade::GOAL.to_string()),
        Some(Grounding::Fix(_)) => Some(fix::GOAL.to_string()),
        None => matches
            .get_many::<String>("goal")
            .map(|words| words.map(String::as_str).collect::<Vec<_>>().join(" ")),
    };
    let cwd = std::env::current_dir()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let input = if reads_stdin(goal.is_some(), &mode) {
        input::Input::stdin()
    } else {
        input::Input::closed()
    };
    runtime.block_on(async {
        let settings = Settings {
            label: description.label,
            notice,
            style,
            mode,
            root,
            cwd,
            prices,
        };
        let mut session = Session::new(&backend, settings, std::io::stdout(), input);
        match (grounding, goal) {
            (Some(Grounding::Upgrade(root, plan)), _) => {
                let brief = upgrade::brief(&root, &plan);
                session.briefed(&plan.summary, brief, upgrade::GOAL).await;
            }
            (Some(Grounding::Fix(prepared)), _) => {
                session
                    .briefed(&prepared.summary, prepared.brief, fix::GOAL)
                    .await;
            }
            (None, Some(goal)) => session.one_shot(&goal).await,
            (None, None) => session.repl().await,
        }
    });
    Ok(())
}

#[cfg(test)]
#[path = "tests/dispatch.rs"]
mod tests;
