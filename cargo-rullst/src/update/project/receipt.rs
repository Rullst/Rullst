//! Local verification evidence is revalidated, never treated as an apply token.
use super::{ProjectError, snapshot, state::State, verify};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Verified {
    schema_version: String,
    phase: String,
    pub prepared_directory: PathBuf,
    verified_directory: PathBuf,
    pub verified_candidate_directory: PathBuf,
    source: PathBuf,
    pub target: String,
    platform: String,
    features: Vec<String>,
    offline: bool,
    commands: Vec<Observation>,
    pub files: Vec<snapshot::Record>,
    execution_authorized_for_this_invocation: bool,
    application_authorized: bool,
    production_ready: bool,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    program: String,
    args: Vec<String>,
    success: bool,
    stdout: String,
    stdout_sha256: String,
    stderr: String,
    stderr_sha256: String,
}

impl Verified {
    pub fn load(directory: &Path) -> Result<(Self, String), ProjectError> {
        let bytes = snapshot::read(directory, "verification.json")?
            .ok_or(ProjectError::Invalid("missing verification record"))?;
        let verified: Self = serde_json::from_slice(&bytes)?;
        if verified.schema_version != "rullst.project-verification.v1"
            || verified.phase != "verified"
            || verified.platform != std::env::consts::OS
            || !verified.execution_authorized_for_this_invocation
            || verified.application_authorized
            || verified.production_ready
            || verified.verified_directory != directory
            || verified.verified_candidate_directory != directory.join("candidate")
            || verified.prepared_directory == directory
            || verified.files.len() > snapshot::MAX_ENTRIES
        {
            return Err(ProjectError::Invalid("invalid project verification record"));
        }
        let args = ["verify", "--prepared", ".", "--dry-run"]
            .into_iter()
            .map(String::from)
            .chain(verified.features.iter().cloned());
        let flags = verify::command()
            .try_get_matches_from(args)
            .map_err(|_| ProjectError::Invalid("unsupported verification feature policy"))?;
        let features = verify::features(&flags)?;
        if features != verified.features {
            return Err(ProjectError::Invalid(
                "noncanonical verification feature policy",
            ));
        }
        let expected = verify::commands(&verified.features, verified.offline);
        if expected.len() != verified.commands.len() {
            return Err(ProjectError::Invalid(
                "incomplete verification command inventory",
            ));
        }
        for (index, (command, observation)) in expected.iter().zip(&verified.commands).enumerate() {
            if command.program != observation.program
                || command.args != observation.args
                || !observation.success
                || observation.stdout != format!("command-{index}.stdout")
                || observation.stderr != format!("command-{index}.stderr")
            {
                return Err(ProjectError::Invalid(
                    "verification command or status changed",
                ));
            }
            for (name, digest) in [
                (&observation.stdout, &observation.stdout_sha256),
                (&observation.stderr, &observation.stderr_sha256),
            ] {
                let body = snapshot::read(directory, name)?
                    .ok_or(ProjectError::Invalid("missing verification log"))?;
                if body.len() > 8 * 1024 * 1024 || hex::encode(Sha256::digest(&body)) != *digest {
                    return Err(ProjectError::Invalid(
                        "verification log changed or exceeds its byte bound",
                    ));
                }
            }
        }
        Ok((verified, hex::encode(Sha256::digest(&bytes))))
    }

    pub fn validate(&self, state: &State) -> Result<(), ProjectError> {
        if self.source != state.prepared.source
            || self.target != state.prepared.target
            || self.files.len() != state.candidate_records.len()
        {
            return Err(ProjectError::Invalid(
                "verification scope differs from preparation",
            ));
        }
        let root = &self.verified_candidate_directory;
        let mut seen = BTreeSet::new();
        let mut present = BTreeSet::new();
        let mut total = 0u64;
        for (file, input) in self.files.iter().zip(&state.candidate_records) {
            snapshot::validate_path(&file.path)?;
            total = total
                .checked_add(file.bytes)
                .ok_or(ProjectError::Invalid("verified source size overflow"))?;
            if total > snapshot::MAX_TOTAL
                || !seen.insert(file.path.clone())
                || file.path != input.path
            {
                return Err(ProjectError::Invalid("invalid verification file inventory"));
            }
            if file.sha256.is_some() {
                present.insert(file.path.clone());
            }
            let body = snapshot::read(root, &file.path)?;
            if snapshot::record(&file.path, body.as_deref()) != *file
                || (file.path != "Cargo.lock" && file != input)
            {
                return Err(ProjectError::Invalid("verified candidate bytes changed"));
            }
        }
        if snapshot::tree(root)? != present {
            return Err(ProjectError::Invalid(
                "verified candidate inventory changed",
            ));
        }
        crate::generators::build::validate_prepared_resolution(root, &self.target)
            .map_err(|error| ProjectError::Planning(error.to_string()))
    }
}
