use super::{ProjectError, process, receipt::Verified, snapshot, state::State};
use clap::{Arg, ArgAction, ArgMatches, Command};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(super) fn command() -> Command {
    Command::new("review")
        .about("Revalidate a verified candidate and display its complete dependency diff")
        .arg(
            Arg::new("verified")
                .long("verified")
                .required(true)
                .value_name("DIRECTORY")
                .value_parser(clap::value_parser!(PathBuf)),
        )
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(super) fn run(matches: &ArgMatches) -> Result<(), ProjectError> {
    let requested = matches
        .get_one::<PathBuf>("verified")
        .ok_or(ProjectError::Invalid("a verified directory is required"))?;
    let verified_lock = super::super::cache::open_project(requested)?;
    let (verified, receipt_digest) = Verified::load(&verified_lock.path)?;
    let prepared_lock = super::super::cache::open_project(&verified.prepared_directory)?;
    let state = State::load(&prepared_lock.path)?;
    verified.validate(&state)?;
    let changes: Vec<_> = state
        .prepared
        .files
        .iter()
        .zip(&verified.files)
        .filter(|(before, after)| *before != *after)
        .map(|(before, after)| Change {
            before: before.clone(),
            after: after.clone(),
        })
        .collect();
    let diff = process::diff(
        &prepared_lock.path.join("before"),
        &verified.verified_candidate_directory,
    )?;
    state.unchanged()?;
    verified.validate(&state)?;
    let proposal = serde_json::json!({"schema_version":"rullst.project-review.v1", "source":state.prepared.source,
        "target":verified.target,"verified_directory":verified_lock.path,"receipt_sha256":receipt_digest,
        "changes":changes,"diff_sha256":hex::encode(Sha256::digest(diff.as_bytes())),"application_authorized":false});
    let digest = hex::encode(Sha256::digest(serde_json::to_vec(&proposal)?));
    let report = serde_json::json!({"review":proposal,"review_sha256":digest,"diff":diff});
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{diff}");
        println!("Review SHA-256: {digest}");
        println!(
            "Application is not authorized by this review. The apply/recovery stage is still in development."
        );
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct Change {
    before: snapshot::Record,
    after: snapshot::Record,
}
