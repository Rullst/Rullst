//! Interactive orchestration of the same checked commands, without a shell.
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::{Value, json};
use std::{ffi::OsString, io::IsTerminal, path::PathBuf, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, thiserror::Error)]
enum GuidedError {
    #[error("guided update rejected: {0}")]
    Invalid(&'static str),
}

#[path = "guided_tests.rs"]
#[cfg(test)]
mod tests;

pub(super) fn command() -> Command {
    Command::new("guided")
        .about("Review and approve a project framework update interactively")
        .arg(Arg::new("to").long("to").required(true).value_name("EXACT_VERSION"))
        .arg(Arg::new("project").long("project").default_value(".").value_parser(clap::value_parser!(PathBuf)))
        .arg(Arg::new("offline").long("offline").action(ArgAction::SetTrue))
        .arg(Arg::new("all-features").long("all-features").action(ArgAction::SetTrue).conflicts_with("features"))
        .arg(Arg::new("features").long("features").value_name("COMMA_SEPARATED"))
        .arg(Arg::new("no-default-features").long("no-default-features").action(ArgAction::SetTrue).conflicts_with("all-features"))
        .arg(Arg::new("timeout-seconds").long("timeout-seconds").default_value("900").value_parser(clap::value_parser!(u64).range(1..=3600)))
        .after_help("Requires interactive input/output. Every approval defaults to no. Use the explicit update project commands and JSON reports for automation. The CLI never replaces itself: install a newer one with `cargo install cargo-rullst --version =VERSION --locked` first. PATH, databases and deployments remain separate.")
}

trait Ui {
    fn show(&mut self, report: &Value) -> Result<()>;
    fn confirm(&mut self, prompt: &str) -> Result<bool>;
}

struct Terminal;
impl Ui for Terminal {
    fn show(&mut self, report: &Value) -> Result<()> {
        // JSON escapes control characters from paths, diffs and provider metadata.
        println!("{}", serde_json::to_string_pretty(report)?);
        Ok(())
    }
    fn confirm(&mut self, prompt: &str) -> Result<bool> {
        Ok(dialoguer::Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?)
    }
}

#[derive(Debug)]
struct Options {
    target: String,
    project: PathBuf,
    verification: Vec<OsString>,
    offline: bool,
}

impl Options {
    fn parse(matches: &ArgMatches) -> Result<Self> {
        Self::parse_with(matches, std::env::var_os("CARGO_NET_OFFLINE").as_deref())
    }

    /// [`Options::parse`] with the value of `CARGO_NET_OFFLINE`.
    fn parse_with(
        matches: &ArgMatches,
        cargo_net_offline: Option<&std::ffi::OsStr>,
    ) -> Result<Self> {
        let target = matches
            .get_one::<String>("to")
            .ok_or(GuidedError::Invalid("exact version required"))?
            .clone();
        let installed = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
        if semver::Version::parse(&target)?.major != installed.major {
            return Err(GuidedError::Invalid("project updates require the target major's migration rules; install that CLI with `cargo install cargo-rullst --version =VERSION --locked`, then run its guided flow").into());
        }
        let mut verification = super::project::verification_features(matches)?
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>();
        verification.extend([
            "--timeout-seconds".into(),
            matches
                .get_one::<u64>("timeout-seconds")
                .ok_or(GuidedError::Invalid("verification deadline required"))?
                .to_string()
                .into(),
        ]);
        let offline = matches.get_flag("offline")
            || crate::ui::update_check::enabled_env_flag(cargo_net_offline);
        Ok(Self {
            target,
            project: matches
                .get_one::<PathBuf>("project")
                .ok_or(GuidedError::Invalid("project path required"))?
                .clone(),
            verification,
            offline,
        })
    }
}

pub(super) fn run(matches: &ArgMatches) -> Result<()> {
    require_terminal(
        std::io::stdin().is_terminal(),
        std::io::stdout().is_terminal(),
    )?;
    let options = Options::parse(matches)?;
    flow(&options, &mut Terminal, execute)
}

/// Prompts need a terminal on both standard input and output.
fn require_terminal(stdin: bool, stdout: bool) -> Result<()> {
    if !stdin || !stdout {
        return Err(GuidedError::Invalid("guided updates require an interactive terminal; use explicit update commands with --json and reviewed approval digests for automation").into());
    }
    Ok(())
}

fn execute(args: &[OsString]) -> Result<Value> {
    let matches = super::command().try_get_matches_from(
        std::iter::once(OsString::from("update")).chain(args.iter().cloned()),
    )?;
    let (action, matches) = matches
        .subcommand()
        .ok_or(GuidedError::Invalid("update operation required"))?;
    match action {
        "project" => super::project::execute(matches).map_err(Into::into),
        _ => Err(GuidedError::Invalid("unsupported guided operation").into()),
    }
}

fn step(
    ui: &mut impl Ui,
    execute: &mut impl FnMut(&[OsString]) -> Result<Value>,
    args: &[OsString],
) -> Result<Value> {
    ui.show(&json!({"running":arguments(args)?}))?;
    let start = Instant::now();
    let report = execute(args)?;
    ui.show(&json!({"elapsed_ms":start.elapsed().as_millis(),"result":report}))?;
    Ok(report)
}

fn arguments(args: &[OsString]) -> Result<Vec<&str>> {
    args.iter().map(|arg| arg.to_str().ok_or_else(|| GuidedError::Invalid("guided display requires Unicode paths; use explicit commands for native non-Unicode paths").into())).collect()
}

fn value(report: &Value, name: &str) -> Result<OsString> {
    Ok(report[name]
        .as_str()
        .ok_or(GuidedError::Invalid("missing command result field"))?
        .into())
}

fn stopped(ui: &mut impl Ui) -> Result<()> {
    ui.show(&json!({"stopped":true,"message":"No further step was authorized. Completed steps and their recovery records are retained."}))
}

fn flow(
    options: &Options,
    ui: &mut impl Ui,
    mut execute: impl FnMut(&[OsString]) -> Result<Value>,
) -> Result<()> {
    ui.show(&json!({"version":options.target,"project":options.project,
        "release_notes":format!("https://github.com/Rullst/Rullst/releases/tag/v{}",options.target),
        "path_changes":false,"database_changes":false,"deployment":false}))?;
    if !ui.confirm("Copy the selected Git project's working files into private storage and prepare its dependency plan?")? { return stopped(ui); }
    let prepared = step(
        ui,
        &mut execute,
        &[
            "project".into(),
            "prepare".into(),
            "--project".into(),
            options.project.as_os_str().to_owned(),
            "--to".into(),
            options.target.clone().into(),
        ],
    )?;
    let mut verify = vec![
        "project".into(),
        "verify".into(),
        "--prepared".into(),
        value(&prepared, "prepared_directory")?,
    ];
    verify.extend(options.verification.clone());
    if !options.offline
        && ui.confirm(
            "Allow Cargo network access during the following verification? No keeps Cargo offline.",
        )?
    {
        verify.push("--allow-network".into());
    }
    let mut preview = verify.clone();
    preview.push("--dry-run".into());
    step(ui, &mut execute, &preview)?;
    if !ui.confirm("Execute these Cargo commands for this trusted project? Build scripts, macros and tests run with your permissions; the copy is not a sandbox.")? { return stopped(ui); }
    verify.push("--allow-project-code".into());
    let verified = step(ui, &mut execute, &verify)?;
    let directory = value(&verified, "verified_directory")?;
    let review = step(
        ui,
        &mut execute,
        &[
            "project".into(),
            "review".into(),
            "--verified".into(),
            directory.clone(),
        ],
    )?;
    if !ui.confirm("Approve this complete dependency diff and apply only its reviewed manifests/root lockfile? Stop other source writers first.")? { return stopped(ui); }
    let digest = value(&review, "review_sha256")?;
    let recovery = vec![
        OsString::from("project"),
        "recover".into(),
        "--verified".into(),
        directory.clone(),
        "--approved-review".into(),
        digest.clone(),
    ];
    ui.show(&json!({"recovery_args":arguments(&recovery)?}))?;
    step(
        ui,
        &mut execute,
        &[
            "project".into(),
            "apply".into(),
            "--verified".into(),
            directory,
            "--approved-review".into(),
            digest,
        ],
    )?;
    ui.show(&json!({"completed":"project","version":options.target,"deployment_performed":false}))
}
