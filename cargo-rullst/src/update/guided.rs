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
        .about("Review and approve a CLI and/or project update interactively")
        .arg(Arg::new("to").long("to").required(true).value_name("EXACT_VERSION"))
        .arg(Arg::new("scope").long("scope").value_parser(["cli", "project", "both"]).default_value("both"))
        .arg(Arg::new("root").long("root").value_parser(clap::value_parser!(PathBuf))
            .required_if_eq_any([("scope", "cli"), ("scope", "both")]).value_name("ABSOLUTE_INSTALLATION_DIRECTORY"))
        .arg(Arg::new("project").long("project").default_value(".").value_parser(clap::value_parser!(PathBuf)))
        .arg(Arg::new("allow-major").long("allow-major").action(ArgAction::SetTrue))
        .arg(Arg::new("prerelease").long("prerelease").action(ArgAction::SetTrue))
        .arg(Arg::new("offline").long("offline").action(ArgAction::SetTrue))
        .arg(Arg::new("all-features").long("all-features").action(ArgAction::SetTrue).conflicts_with("features"))
        .arg(Arg::new("features").long("features").value_name("COMMA_SEPARATED"))
        .arg(Arg::new("no-default-features").long("no-default-features").action(ArgAction::SetTrue).conflicts_with("all-features"))
        .arg(Arg::new("timeout-seconds").long("timeout-seconds").default_value("900").value_parser(clap::value_parser!(u64).range(1..=3600)))
        .after_help("Requires interactive input/output. Every approval defaults to no. Use the explicit stage/install/project commands and JSON reports for automation. Installing a CLI does not change this running process's migration rules. PATH, databases and deployments remain separate.")
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
    scope: String,
    root: Option<PathBuf>,
    project: PathBuf,
    selection: Vec<OsString>,
    verification: Vec<OsString>,
    offline: bool,
}

impl Options {
    fn parse(matches: &ArgMatches) -> Result<Self> {
        let target = matches
            .get_one::<String>("to")
            .ok_or(GuidedError::Invalid("exact version required"))?
            .clone();
        let installed = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
        super::catalog::Selection::new(
            &installed,
            Some(&target),
            matches.get_flag("allow-major"),
            matches.get_flag("prerelease"),
        )?;
        let scope = matches
            .get_one::<String>("scope")
            .ok_or(GuidedError::Invalid("scope required"))?
            .clone();
        if scope != "cli" && semver::Version::parse(&target)?.major != installed.major {
            return Err(GuidedError::Invalid("project updates require this CLI's major-specific migration rules; install the selected CLI with --scope cli, then invoke its project flow explicitly").into());
        }
        let selection = ["allow-major", "prerelease"]
            .into_iter()
            .filter(|flag| matches.get_flag(flag))
            .map(|flag| OsString::from(format!("--{flag}")))
            .collect();
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
            || crate::ui::update_check::enabled_env_flag(
                std::env::var_os("CARGO_NET_OFFLINE").as_deref(),
            );
        if scope != "project" && offline {
            return Err(GuidedError::Invalid("offline mode forbids authenticated CLI installation; use --scope project for offline project verification").into());
        }
        Ok(Self {
            target,
            scope,
            root: matches.get_one::<PathBuf>("root").cloned(),
            project: matches
                .get_one::<PathBuf>("project")
                .ok_or(GuidedError::Invalid("project path required"))?
                .clone(),
            selection,
            verification,
            offline,
        })
    }
}

pub(super) fn run(matches: &ArgMatches) -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(GuidedError::Invalid("guided updates require an interactive terminal; use explicit update commands with --json and reviewed approval digests for automation").into());
    }
    let options = Options::parse(matches)?;
    flow(&options, &mut Terminal, execute)
}

fn execute(args: &[OsString]) -> Result<Value> {
    let matches = super::command().try_get_matches_from(
        std::iter::once(OsString::from("update")).chain(args.iter().cloned()),
    )?;
    let (action, matches) = matches
        .subcommand()
        .ok_or(GuidedError::Invalid("update operation required"))?;
    match action {
        "stage" | "install" => super::artifacts::execute(action, matches),
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
    ui.show(&json!({"scope":options.scope,"version":options.target,"installation_root":options.root,"project":options.project,
        "release_notes":format!("https://github.com/Rullst/Rullst/releases/tag/v{}",options.target),
        "path_changes":false,"database_changes":false,"deployment":false}))?;
    if options.scope != "project" {
        let root = options
            .root
            .as_ref()
            .ok_or(GuidedError::Invalid("installation root required"))?;
        if !ui.confirm("Download and authenticate this exact CLI release using registry/GitHub network access and private storage?")? { return stopped(ui); }
        let mut stage = vec!["stage".into(), "--to".into(), options.target.clone().into()];
        stage.extend(options.selection.clone());
        let staged = step(ui, &mut execute, &stage)?;
        let mut install = vec![
            "install".into(),
            "review".into(),
            "--to".into(),
            options.target.clone().into(),
            "--directory".into(),
            value(&staged, "directory")?,
            "--root".into(),
            root.as_os_str().to_owned(),
        ];
        install.extend(options.selection.clone());
        let review = step(ui, &mut execute, &install)?;
        if !ui.confirm("Approve this complete installation review: execute its two bounded version probes and replace its recorded private CLI entries?")? { return stopped(ui); }
        install[1] = "apply".into();
        install.extend(["--approved-review".into(), value(&review, "review_sha256")?]);
        step(ui, &mut execute, &install)?;
    }
    if options.scope != "cli" {
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
        if !options.offline && ui.confirm("Allow Cargo network access during the following verification? No keeps Cargo offline.")? { verify.push("--allow-network".into()); }
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
    }
    ui.show(&json!({"completed_scope":options.scope,"version":options.target,"deployment_performed":false}))
}
