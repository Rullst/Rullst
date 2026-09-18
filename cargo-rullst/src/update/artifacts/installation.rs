//! Explicit installation review. Application/recovery use separate entry points.
use super::{ArtifactError, files, manifest::Manifest, native_target, provenance};
use clap::{Arg, ArgAction, ArgMatches, Command};
use semver::Version;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn command() -> Command {
    Command::new("install").about("Review an authenticated native CLI installation")
        .subcommand_required(true).arg_required_else_help(true)
        .subcommand(Command::new("review").about("Preview a new private installation without executing candidates or editing the destination")
            .arg(Arg::new("directory").long("directory").required(true).value_name("ARTIFACTS").value_parser(clap::value_parser!(PathBuf)))
            .arg(Arg::new("root").long("root").required(true).value_name("ABSOLUTE_INSTALLATION_DIRECTORY").value_parser(clap::value_parser!(PathBuf)))
            .arg(Arg::new("to").long("to").required(true).value_name("EXACT_VERSION"))
            .arg(Arg::new("allow-major").long("allow-major").action(ArgAction::SetTrue))
            .arg(Arg::new("prerelease").long("prerelease").action(ArgAction::SetTrue))
            .arg(Arg::new("offline").long("offline").action(ArgAction::SetTrue))
            .arg(Arg::new("json").long("json").action(ArgAction::SetTrue)))
}

pub(super) fn run(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let matches = matches
        .subcommand_matches("review")
        .ok_or(ArtifactError::Invalid("unsupported installation operation"))?;
    if matches.get_flag("offline")
        || crate::ui::update_check::enabled_env_flag(
            std::env::var_os("CARGO_NET_OFFLINE").as_deref(),
        )
    {
        return Err(ArtifactError::Invalid(
            "offline mode forbids authenticated installation review",
        )
        .into());
    }
    let exact = matches
        .get_one::<String>("to")
        .ok_or(ArtifactError::Invalid("exact version required"))?;
    let installed = Version::parse(env!("CARGO_PKG_VERSION"))?;
    let policy = super::super::catalog::Selection::new(
        &installed,
        Some(exact),
        matches.get_flag("allow-major"),
        matches.get_flag("prerelease"),
    )?;
    let version = Version::parse(exact)?;
    let target = native_target()?;
    let directory = matches
        .get_one::<PathBuf>("directory")
        .ok_or(ArtifactError::Invalid("artifact directory required"))?;
    let requested = matches
        .get_one::<PathBuf>("root")
        .ok_or(ArtifactError::Invalid("installation root required"))?;
    let root = new_destination(requested)?;
    let registry = super::super::fetch_catalog()?;
    super::super::catalog::resolve(&registry, &installed, &policy)?;
    files::directory(directory)?;
    let body = files::read_bounded(
        &directory.join(format!("cli-manifest-{target}.json")),
        16 * 1024,
    )?;
    let artifact = Manifest::parse(&body, &version, target)?;
    let private = super::super::cache::verification_manifest(&body)?;
    provenance::verify(private.path(), &artifact)?;
    artifact.verify_files(directory)?;
    // Destination state is rechecked after the network/verifier work.
    if new_destination(requested)? != root {
        return Err(
            ArtifactError::Invalid("installation destination changed during review").into(),
        );
    }
    let report = review(&root, &artifact)?;
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Reviewed CLI {version} for {target}: {}", root.display());
        println!(
            "Installation review: {}",
            report["review_sha256"].as_str().unwrap_or("unavailable")
        );
        println!(
            "No candidate was executed or installed. Apply/recovery acceptance is still under development."
        );
        println!(
            "For an existing Cargo-owned installation, use the pinned command in the JSON plan after reviewing source compilation."
        );
    }
    Ok(())
}

fn new_destination(requested: &Path) -> Result<PathBuf, ArtifactError> {
    let root = super::super::cache::installation_root(requested)?;
    if root.try_exists()? && fs::read_dir(&root)?.next().transpose()?.is_some() {
        return Err(ArtifactError::Invalid(
            "destination is not empty; existing binaries and package-manager installations cannot be taken over",
        ));
    }
    Ok(root)
}

fn review(root: &Path, artifact: &Manifest) -> Result<serde_json::Value, ArtifactError> {
    let suffix = if artifact.target.ends_with("windows-msvc") {
        ".exe"
    } else {
        ""
    };
    let plan = serde_json::json!({"schema_version":"rullst.cli-installation-review.v1", "root":root,
        "artifact":artifact,"prior_installation":null,"proposed_version_checks":[
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
    #[test]
    fn proposed_binary_execution_and_source_fallback_grant_no_write_authority() {
        let manifest = super::super::tests::fixture_manifest();
        let directory = tempfile::tempdir().unwrap();
        let report = review(directory.path(), &manifest).unwrap();
        assert_eq!(report, review(directory.path(), &manifest).unwrap());
        assert_eq!(
            report["review"]["authority"]["cli_installation_authorized"],
            false
        );
        assert_eq!(report["review"]["authority"]["candidate_executed"], false);
        assert_eq!(report["review"]["source_fallback"]["args"][3], "=12.1.0");
        let changed = review(&directory.path().join("another"), &manifest).unwrap();
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
