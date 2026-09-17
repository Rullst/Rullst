use super::{ProjectError, process, receipt::Verified, snapshot, state::State};
use clap::{Arg, ArgAction, ArgMatches, Command};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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
    let evidence = Evidence::load(requested)?;
    let report = serde_json::json!({"review":evidence.proposal,"review_sha256":evidence.digest,"diff":evidence.diff});
    if matches.get_flag("json") {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", evidence.diff);
        println!("Review SHA-256: {}", evidence.digest);
        println!("Application is not authorized by this review.");
    }
    Ok(())
}

pub(super) struct Evidence {
    pub verified_lock: super::super::cache::LockedProject,
    pub prepared_lock: super::super::cache::LockedProject,
    pub state: State,
    pub verified: Verified,
    pub receipt_digest: String,
    pub changes: Vec<Change>,
    pub permissions: Vec<super::transaction::Permissions>,
    pub proposal: serde_json::Value,
    pub diff: String,
    pub digest: String,
}

impl Evidence {
    pub fn load(requested: &Path) -> Result<Self, ProjectError> {
        Self::load_inner(requested, false)
    }

    pub fn load_for_recovery(requested: &Path) -> Result<Self, ProjectError> {
        Self::load_inner(requested, true)
    }

    fn load_inner(requested: &Path, recovery: bool) -> Result<Self, ProjectError> {
        let verified_lock = super::super::cache::open_project(requested)?;
        let (verified, receipt_digest) = Verified::load(&verified_lock.path)?;
        let prepared_lock = super::super::cache::open_project(&verified.prepared_directory)?;
        let state = if recovery {
            State::load_for_recovery(&prepared_lock.path)?
        } else {
            State::load(&prepared_lock.path)?
        };
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
        let permissions = changes
            .iter()
            .map(|change| {
                super::transaction::Permissions::capture(&state.prepared.source, &change.before)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let diff = process::diff(
            &prepared_lock.path.join("before"),
            &verified.verified_candidate_directory,
        )?;
        if !recovery {
            state.unchanged()?;
        }
        verified.validate(&state)?;
        let proposal = serde_json::json!({"schema_version":"rullst.project-review.v1", "source":state.prepared.source,
        "target":verified.target,"verified_directory":verified_lock.path,"receipt_sha256":receipt_digest,
        "changes":changes,"permissions":permissions,"diff_sha256":hex::encode(Sha256::digest(diff.as_bytes())),"application_authorized":false});
        let digest = hex::encode(Sha256::digest(serde_json::to_vec(&proposal)?));
        Ok(Self {
            verified_lock,
            prepared_lock,
            state,
            verified,
            receipt_digest,
            changes,
            permissions,
            proposal,
            diff,
            digest,
        })
    }

    pub fn revalidate(&self) -> Result<(), ProjectError> {
        self.revalidate_ignoring(&std::collections::BTreeSet::new())
    }

    pub fn revalidate_ignoring(
        &self,
        owned_staging: &std::collections::BTreeSet<String>,
    ) -> Result<(), ProjectError> {
        self.state.unchanged_ignoring(owned_staging)?;
        for (change, expected) in self.changes.iter().zip(&self.permissions) {
            if super::transaction::Permissions::capture(
                &self.state.prepared.source,
                &change.before,
            )? != *expected
            {
                return Err(ProjectError::Invalid(
                    "source permissions changed after review",
                ));
            }
        }
        let (_, digest) = Verified::load(&self.verified_lock.path)?;
        if digest != self.receipt_digest {
            return Err(ProjectError::Invalid(
                "verification record changed after review",
            ));
        }
        self.verified.validate(&self.state)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Change {
    pub before: snapshot::Record,
    pub after: snapshot::Record,
}
