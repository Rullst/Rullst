//! Explicit installation review. Application/recovery use separate entry points.
use super::{ArtifactError, files, manifest::Manifest, native_target, provenance};
use clap::{Arg, ArgAction, ArgMatches, Command};
use semver::Version;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[path = "installation/application.rs"]
mod application;
#[cfg(test)]
#[path = "installation/tests.rs"]
mod native_tests;
#[path = "installation/recovery.rs"]
mod recovery;
#[path = "installation/retention.rs"]
mod retention;
#[path = "installation/smoke.rs"]
mod smoke;
#[path = "installation/state.rs"]
mod state;
#[path = "installation/storage.rs"]
mod storage;
#[path = "installation/transaction.rs"]
mod transaction;

pub(super) fn command() -> Command {
    let flags = |command: Command| {
        command
            .arg(
                Arg::new("root")
                    .long("root")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf))
                    .value_name("ABSOLUTE_INSTALLATION_DIRECTORY"),
            )
            .arg(
                Arg::new("offline")
                    .long("offline")
                    .action(ArgAction::SetTrue),
            )
            .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
    };
    let candidate = |command: Command| {
        flags(command)
            .arg(
                Arg::new("directory")
                    .long("directory")
                    .required(true)
                    .value_parser(clap::value_parser!(PathBuf)),
            )
            .arg(
                Arg::new("to")
                    .long("to")
                    .required(true)
                    .value_name("EXACT_VERSION"),
            )
            .arg(
                Arg::new("allow-major")
                    .long("allow-major")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("prerelease")
                    .long("prerelease")
                    .action(ArgAction::SetTrue),
            )
    };
    let approval = || {
        Arg::new("approved-review")
            .long("approved-review")
            .required(true)
            .value_name("SHA256")
    };
    Command::new("install")
        .about("Review, apply or recover an authenticated private CLI installation")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(candidate(Command::new("review").about(
            "Authenticate and preview without executing or installing candidates",
        )))
        .subcommand(
            candidate(
                Command::new("apply")
                    .about("Run the approved version probes and install the reviewed binaries"),
            )
            .arg(approval()),
        )
        .subcommand(
            flags(Command::new("recover").about(
                "Restore only the recorded predecessor of the current installation operation",
            ))
            .arg(approval()),
        )
}

pub(super) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let (action, matches) = matches
        .subcommand()
        .ok_or(ArtifactError::Invalid("installation operation required"))?;
    if matches.get_flag("offline")
        || crate::ui::update_check::enabled_env_flag(
            std::env::var_os("CARGO_NET_OFFLINE").as_deref(),
        )
    {
        return Err(ArtifactError::Invalid(
            "offline mode forbids authenticated installation operations",
        )
        .into());
    }
    let approved = if action == "review" {
        None
    } else {
        let digest = matches
            .get_one::<String>("approved-review")
            .ok_or(ArtifactError::Invalid("explicit reviewed digest required"))?;
        if !transaction::valid_digest(digest) {
            return Err(ArtifactError::Invalid(
                "approval must be a lowercase SHA-256 review digest",
            )
            .into());
        }
        Some(digest.as_str())
    };
    let requested = matches
        .get_one::<PathBuf>("root")
        .ok_or(ArtifactError::Invalid("installation root required"))?;
    let root = crate::update::cache::installation_root(requested)?;
    let report = if action == "recover" {
        application::recover(
            &root,
            approved.ok_or(ArtifactError::Invalid("recovery approval required"))?,
        )?
    } else {
        let candidate = inspect(matches, root)?;
        let report = review(
            &candidate.root,
            &candidate.artifact,
            candidate.prior.as_ref(),
        )?;
        if action == "review" {
            report
        } else if action == "apply" {
            let approved =
                approved.ok_or(ArtifactError::Invalid("installation approval required"))?;
            if report["review_sha256"].as_str() != Some(approved) {
                return Err(ArtifactError::Invalid(
                    "review changed or approval does not match; run install review again",
                )
                .into());
            }
            application::apply(&candidate, approved)?
        } else {
            return Err(ArtifactError::Invalid("unsupported installation operation").into());
        }
    };
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if action == "review" {
        println!(
            "Installation review: {}",
            report["review_sha256"].as_str().unwrap_or("unavailable")
        );
        println!(
            "No binary was executed or installed. Inspect --json for the destination, exact candidate and proposed --version probes."
        );
        println!(
            "Apply requires --approved-review with this digest. Existing package-manager installations require their pinned source/manager fallback."
        );
    } else {
        println!(
            "CLI installation {action} completed. {}",
            serde_json::to_string(&report)?
        );
    }
    Ok(())
}

struct Candidate {
    root: PathBuf,
    directory: PathBuf,
    artifact: Manifest,
    raw_manifest: Vec<u8>,
    prior: Option<state::Installed>,
}

fn inspect(matches: &ArgMatches, root: PathBuf) -> Result<Candidate, ArtifactError> {
    let exact = matches
        .get_one::<String>("to")
        .ok_or(ArtifactError::Invalid("exact version required"))?;
    let version = Version::parse(exact).map_err(crate::update::catalog::SelectionError::Version)?;
    let target = native_target()?;
    let directory = matches
        .get_one::<PathBuf>("directory")
        .ok_or(ArtifactError::Invalid("artifact directory required"))?;
    let prior = state::inspect(&root, target)?;
    let installed = match &prior {
        Some(previous) => previous.version()?,
        None => Version::parse(env!("CARGO_PKG_VERSION"))
            .map_err(crate::update::catalog::SelectionError::Version)?,
    };
    let policy = crate::update::catalog::Selection::new(
        &installed,
        Some(exact),
        matches.get_flag("allow-major"),
        matches.get_flag("prerelease"),
    )?;
    if let Some(previous) = &prior {
        authenticate(&previous.raw_manifest, &previous.manifest)?;
    }
    let registry = crate::update::fetch_catalog()?;
    crate::update::catalog::resolve(&registry, &installed, &policy)?;
    files::directory(directory)?;
    let body = files::read_bounded(
        &directory.join(format!("cli-manifest-{target}.json")),
        16 * 1024,
    )?;
    let artifact = Manifest::parse(&body, &version, target)?;
    authenticate(&body, &artifact)?;
    artifact.verify_files(directory)?;
    let candidate = Candidate {
        root,
        directory: directory.clone(),
        artifact,
        raw_manifest: body,
        prior,
    };
    unchanged(&candidate)?;
    Ok(candidate)
}

fn authenticate(body: &[u8], manifest: &Manifest) -> Result<(), ArtifactError> {
    let private = crate::update::cache::verification_manifest(body)?;
    provenance::verify(private.path(), manifest)
}

fn unchanged(candidate: &Candidate) -> Result<(), ArtifactError> {
    if crate::update::cache::installation_root(&candidate.root)? != candidate.root
        || state::inspect(&candidate.root, &candidate.artifact.target)?
            .as_ref()
            .map(state::Installed::summary)
            != candidate.prior.as_ref().map(state::Installed::summary)
    {
        return Err(ArtifactError::Invalid(
            "installation destination changed after review",
        ));
    }
    Ok(())
}

fn review(
    root: &Path,
    artifact: &Manifest,
    prior: Option<&state::Installed>,
) -> Result<serde_json::Value, ArtifactError> {
    let suffix = if artifact.target.ends_with("windows-msvc") {
        ".exe"
    } else {
        ""
    };
    let plan = serde_json::json!({"schema_version":"rullst.cli-installation-review.v1", "root":root,
        "artifact":artifact,"prior_installation":prior.map(state::Installed::summary),"proposed_version_checks":[
            [format!("cargo-rullst{suffix}"),"--version"],[format!("rullst{suffix}"),"--version"]],
        "source_fallback":{"program":"cargo","args":["install","cargo-rullst","--version",format!("={}",artifact.version),"--locked"],"requires_source_execution_consent":true},
        "authority":{"artifact_verified":true,"registry_eligibility_checked":true,"candidate_executed":false,
        "cli_installation_authorized":false,"project_changes_authorized":false,"deployment_authorized":false}});
    let digest = hex::encode(Sha256::digest(serde_json::to_vec(&plan)?));
    Ok(serde_json::json!({"review":plan,"review_sha256":digest}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn new_destination(path: &Path) -> Result<PathBuf, ArtifactError> {
        let root = super::super::super::cache::installation_root(path)?;
        if state::inspect(&root, native_target()?)?.is_some() {
            return Err(ArtifactError::Invalid("expected new destination"));
        }
        Ok(root)
    }

    #[test]
    fn proposed_binary_execution_and_source_fallback_grant_no_write_authority() {
        let manifest = super::super::tests::fixture_manifest();
        let directory = tempfile::tempdir().unwrap();
        let report = review(directory.path(), &manifest, None).unwrap();
        assert_eq!(report, review(directory.path(), &manifest, None).unwrap());
        assert_eq!(
            report["review"]["authority"]["cli_installation_authorized"],
            false
        );
        assert_eq!(report["review"]["authority"]["candidate_executed"], false);
        assert_eq!(report["review"]["source_fallback"]["args"][3], "=12.1.0");
        let changed = review(&directory.path().join("another"), &manifest, None).unwrap();
        assert_ne!(report["review_sha256"], changed["review_sha256"]);
    }
    #[cfg(unix)]
    #[test]
    fn a_new_private_destination_never_takes_over_files_or_creates_directories() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let temporary = tempfile::tempdir().unwrap();
        let base = temporary.path().canonicalize().unwrap();
        fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).unwrap();
        let destination = base.join("install");
        assert_eq!(new_destination(&destination).unwrap(), destination);
        assert!(!destination.exists());
        fs::create_dir(&destination).unwrap();
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o700)).unwrap();
        new_destination(&destination).unwrap();
        fs::write(
            destination.join("cargo-rullst"),
            b"owned by another installer",
        )
        .unwrap();
        assert!(new_destination(&destination).is_err());
        let link = base.join("alias");
        symlink(&destination, &link).unwrap();
        assert!(new_destination(&link).is_err());
        assert_eq!(
            fs::read(destination.join("cargo-rullst")).unwrap(),
            b"owned by another installer"
        );
    }
    #[cfg(windows)]
    #[test]
    fn windows_installation_preview_validates_a_new_destination_without_creating_it() {
        let base = tempfile::tempdir_in(std::env::var_os("LOCALAPPDATA").unwrap()).unwrap();
        let parent = base.path().canonicalize().unwrap();
        let root = parent.join("installation");
        assert_eq!(new_destination(&root).unwrap(), root);
        assert!(!root.exists());
        let existing = parent.join("already-managed");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("cargo-rullst.exe"), b"owned elsewhere").unwrap();
        assert!(new_destination(&existing).is_err());
        assert_eq!(
            fs::read(existing.join("cargo-rullst.exe")).unwrap(),
            b"owned elsewhere"
        );
    }
}
