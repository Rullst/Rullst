use super::{
    ProjectError,
    review::Evidence,
    snapshot,
    transaction::{Intent, Replacement, Staged},
};
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

pub(super) fn command(name: &'static str) -> Command {
    Command::new(name)
        .about(if name == "apply" { "Apply the exact approved manifest/lockfile review; stop other writers first" }
            else { "Restore only this approved operation's manifest/lockfile changes; refuse divergent edits" })
        .arg(Arg::new("verified").long("verified").required(true).value_name("DIRECTORY").value_parser(clap::value_parser!(PathBuf)))
        .arg(Arg::new("approved-review").long("approved-review").required(true).value_name("SHA256"))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(super) fn run(matches: &ArgMatches, recovery: bool) -> Result<(), ProjectError> {
    let path = matches
        .get_one::<PathBuf>("verified")
        .ok_or(ProjectError::Invalid("a verified directory is required"))?;
    let approval = matches
        .get_one::<String>("approved-review")
        .ok_or(ProjectError::Invalid(
            "explicit review approval is required",
        ))?;
    if approval.len() != 64
        || !approval
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProjectError::Invalid(
            "approval must be the complete lowercase SHA-256 from review",
        ));
    }
    let evidence = if recovery {
        Evidence::load_for_recovery(path)?
    } else {
        Evidence::load(path)?
    };
    if approval != &evidence.digest {
        return Err(ProjectError::Invalid(
            "approved review differs from the current evidence/diff; review again",
        ));
    }
    let _source_lock = super::super::cache::source_lock(&evidence.state.prepared.source)?;
    let completed = if recovery {
        recover(&evidence)?
    } else {
        apply(&evidence)?
    };
    let report = serde_json::json!({"schema_version":"rullst.project-application.v1", "phase":if recovery { "restored" } else { "applied" },
        "source":evidence.state.prepared.source,"review_sha256":approval,"file_operations":completed,
        "intent":evidence.verified_lock.path.join("application.json"),"deployment_authorized":false,"database_changes":false});
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "{} {completed} reviewed file operations. Recovery record: {}",
            if recovery { "Restored" } else { "Applied" },
            evidence
                .verified_lock
                .path
                .join("application.json")
                .display()
        );
        println!("Database and external effects were not reversed. Deployment remains separate.");
    }
    Ok(())
}

fn apply(evidence: &Evidence) -> Result<usize, ProjectError> {
    evidence.revalidate()?;
    if snapshot::read(&evidence.verified_lock.path, "application.json")?.is_some() {
        return Err(ProjectError::Invalid(
            "this operation already has an application intent; recover it or prepare a new update",
        ));
    }
    let root = &evidence.state.prepared.source;
    let mut replacements = Vec::new();
    for (change, mode) in evidence.changes.iter().zip(&evidence.permissions) {
        validate_scope(change)?;
        replacements.push(Replacement {
            current: change.before.clone(),
            desired: change.after.clone(),
            permissions: mode.clone(),
        });
    }
    let staged = Staged::create(
        root,
        &evidence.verified.verified_candidate_directory,
        replacements,
    )?;
    evidence.revalidate_ignoring(&staged.owned_paths)?;
    let mut intent = Intent {
        schema_version: "rullst.project-application-intent.v1".into(),
        phase: "applying".into(),
        source: root.clone(),
        prepared_directory: evidence.prepared_lock.path.clone(),
        verified_directory: evidence.verified_lock.path.clone(),
        receipt_sha256: evidence.receipt_digest.clone(),
        review_sha256: evidence.digest.clone(),
        changes: evidence.changes.clone(),
        permissions: evidence.permissions.clone(),
    };
    intent.store(&evidence.verified_lock.path)?;
    finish(staged, &mut intent)
}

fn recover(evidence: &Evidence) -> Result<usize, ProjectError> {
    let bytes = snapshot::read(&evidence.verified_lock.path, "application.json")?.ok_or(
        ProjectError::Invalid("no application intent exists for this verification"),
    )?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(ProjectError::Invalid("application intent exceeds 8 MiB"));
    }
    let mut intent: Intent = serde_json::from_slice(&bytes)?;
    if intent.schema_version != "rullst.project-application-intent.v1"
        || !matches!(
            intent.phase.as_str(),
            "applying" | "applied" | "restoring" | "restored"
        )
        || intent.source != evidence.state.prepared.source
        || intent.prepared_directory != evidence.prepared_lock.path
        || intent.verified_directory != evidence.verified_lock.path
        || intent.receipt_sha256 != evidence.receipt_digest
        || intent.review_sha256 != evidence.digest
        || intent.changes != evidence.changes
        || intent.permissions != evidence.permissions
    {
        return Err(ProjectError::Invalid(
            "application intent does not match the reviewed operation",
        ));
    }
    let mut replacements = Vec::new();
    for (change, mode) in intent.changes.iter().zip(&intent.permissions) {
        validate_scope(change)?;
        let bytes = snapshot::read(&intent.source, &change.before.path)?;
        let current = snapshot::record(&change.before.path, bytes.as_deref());
        if current == change.before {
            continue;
        }
        if current != change.after {
            return Err(ProjectError::Invalid(
                "recovery refuses a divergent user edit; no files were restored",
            ));
        }
        replacements.push(Replacement {
            current,
            desired: change.before.clone(),
            permissions: mode.clone(),
        });
    }
    let staged = Staged::create(
        &intent.source,
        &evidence.prepared_lock.path.join("before"),
        replacements,
    )?;
    intent.phase = "restoring".into();
    intent.store(&evidence.verified_lock.path)?;
    finish(staged, &mut intent)
}

fn finish(staged: Staged, intent: &mut Intent) -> Result<usize, ProjectError> {
    let completed = staged
        .commit(&intent.source)
        .map_err(|(completed, error)| ProjectError::Apply {
            completed,
            reason: error.to_string(),
            directory: intent.verified_directory.clone(),
        })?;
    intent.phase = if intent.phase == "restoring" {
        "restored"
    } else {
        "applied"
    }
    .into();
    intent
        .store(&intent.verified_directory)
        .map_err(|error| ProjectError::Apply {
            completed,
            reason: error.to_string(),
            directory: intent.verified_directory.clone(),
        })?;
    Ok(completed)
}

fn validate_scope(change: &super::review::Change) -> Result<(), ProjectError> {
    let name = &change.before.path;
    if name != &change.after.path
        || (name != "Cargo.lock"
            && std::path::Path::new(name)
                .file_name()
                .is_none_or(|file| file != "Cargo.toml"))
    {
        return Err(ProjectError::Invalid(
            "application is limited to reviewed manifests and the root lockfile",
        ));
    }
    Ok(())
}
