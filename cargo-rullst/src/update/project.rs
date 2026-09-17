//! Opt-in source preparation; execution and application are separate boundaries.
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[path = "project/application.rs"]
mod application;
#[path = "project/process.rs"]
mod process;
#[path = "project/receipt.rs"]
mod receipt;
#[path = "project/review.rs"]
mod review;
#[path = "project/snapshot.rs"]
mod snapshot;
#[path = "project/state.rs"]
mod state;
#[path = "project/transaction.rs"]
mod transaction;
#[path = "project/verify.rs"]
mod verify;

#[derive(thiserror::Error)]
pub(super) enum ProjectError {
    #[error("project preparation rejected: {0}")]
    Invalid(&'static str),
    #[error("project preparation I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("project metadata is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("private project storage failed: {0}")]
    Cache(#[from] super::cache::CacheError),
    #[error("project migration planning failed: {0}")]
    Planning(String),
    #[error(
        "project file operation stopped after {completed} replacements: {reason}; recovery data retained at {directory}"
    )]
    Apply {
        completed: usize,
        reason: String,
        directory: PathBuf,
    },
}

impl std::fmt::Debug for ProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, formatter)
    }
}

pub(super) fn command() -> Command {
    Command::new("project").about("Prepare framework changes in an isolated source copy")
        .subcommand_required(true).subcommand(
            Command::new("prepare").about("Copy a Git project and plan exact dependency changes without running builds or applying them")
                .arg(Arg::new("project").long("project").value_name("PATH").default_value(".")
                    .value_parser(clap::value_parser!(PathBuf)))
                .arg(Arg::new("to").long("to").value_name("EXACT_VERSION"))
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)))
        .subcommand(verify::command())
        .subcommand(review::command())
        .subcommand(application::command("apply"))
        .subcommand(application::command("recover"))
}

pub(super) fn run(matches: &ArgMatches) -> Result<(), ProjectError> {
    if let Some(matches) = matches.subcommand_matches("apply") {
        return application::run(matches, false);
    }
    if let Some(matches) = matches.subcommand_matches("recover") {
        return application::run(matches, true);
    }
    if let Some(matches) = matches.subcommand_matches("review") {
        return review::run(matches);
    }
    if let Some(matches) = matches.subcommand_matches("verify") {
        return verify::run(matches);
    }
    let matches = matches
        .subcommand_matches("prepare")
        .ok_or(ProjectError::Invalid("unsupported project operation"))?;
    let root = matches
        .get_one::<PathBuf>("project")
        .ok_or(ProjectError::Invalid("a project directory is required"))?
        .canonicalize()?;
    let target = matches
        .get_one::<String>("to")
        .map(String::as_str)
        .unwrap_or(env!("CARGO_PKG_VERSION"));
    let parsed = semver::Version::parse(target)
        .map_err(|_| ProjectError::Invalid("use an exact SemVer target"))?;
    if !parsed.build.is_empty() || target.len() > 64 {
        return Err(ProjectError::Invalid(
            "build metadata and oversized versions are unsupported",
        ));
    }
    let paths = snapshot::inventory(&root)?;
    let stage = super::cache::project_workspace()?;
    if stage.path().starts_with(&root) {
        return Err(ProjectError::Invalid(
            "configure private cache storage outside the project",
        ));
    }
    let candidate = stage.path().join("candidate");
    let baseline = stage.path().join("before");
    let records = snapshot::copy(&root, &baseline, &candidate, &paths)?;
    let manifests = manifests(&candidate)?;
    let plan = crate::generators::build::prepare_manifests(&candidate, manifests, target)
        .map_err(|error| ProjectError::Planning(error.to_string()))?;
    snapshot::unchanged(&root, &paths, &records)?;
    let prepared = Prepared {
        schema_version: "rullst.project-preparation.v1".into(),
        phase: "prepared".into(),
        source: root,
        target: target.to_owned(),
        platform: std::env::consts::OS.into(),
        files: records,
        plan,
        execution_authorized: false,
        application_authorized: false,
    };
    let serialized = serde_json::to_vec_pretty(&prepared)?;
    if serialized.len() > 64 * 1024 * 1024 {
        return Err(ProjectError::Invalid("preparation report exceeds 64 MiB"));
    }
    std::fs::write(stage.path().join("preparation.json"), serialized)?;
    let path = stage.retain();
    let report = serde_json::json!({"schema_version":"rullst.project-preparation-result.v1",
        "prepared_directory":path, "candidate_directory":candidate, "preparation":prepared});
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Prepared source copy: {}", path.display());
        println!("Candidate: {}", candidate.display());
        println!(
            "Review preparation.json and compare before/ with candidate/. Original project files were not edited."
        );
        println!(
            "Preparation grants no execution or application authority. Preview checks with update project verify --prepared DIRECTORY --dry-run."
        );
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Prepared {
    schema_version: String,
    phase: String,
    source: PathBuf,
    target: String,
    platform: String,
    files: Vec<snapshot::Record>,
    plan: serde_json::Value,
    execution_authorized: bool,
    application_authorized: bool,
}

#[derive(serde::Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: BTreeSet<String>,
}
#[derive(serde::Deserialize)]
struct Package {
    id: String,
    manifest_path: PathBuf,
}

fn manifests(root: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    let body = process::capture(
        "cargo",
        &[
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ],
        root,
        8 * 1024 * 1024,
    )?;
    let metadata: Metadata = serde_json::from_slice(&body)?;
    let mut paths: BTreeSet<_> = metadata
        .packages
        .into_iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .map(|package| package.manifest_path.canonicalize())
        .collect::<Result<_, _>>()?;
    paths.insert(root.join("Cargo.toml").canonicalize()?);
    Ok(paths.into_iter().collect())
}
