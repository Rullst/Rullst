use super::{
    ArtifactError, Candidate, authenticate, recovery, smoke, state,
    storage::Storage,
    transaction::{self, Intent, Phase},
    unchanged,
};
use crate::update::cache;
use std::path::Path;

pub(super) fn apply(
    candidate: &Candidate,
    approved: &str,
) -> Result<serde_json::Value, ArtifactError> {
    let storage = Storage::open(&candidate.root)?;
    unchanged(candidate)?;
    let (operation, mut intent) = transaction::prepare(
        &storage,
        &candidate.root,
        &candidate.directory,
        &candidate.artifact,
        &candidate.raw_manifest,
        candidate.prior.as_ref(),
        approved,
    )?;
    // Approval covers exactly these bounded probes of authenticated private copies.
    smoke::verify(&operation.join("after"), &candidate.artifact)?;
    unchanged(candidate)?;
    recovery::validate(&operation, &intent)?;
    cache::create_installation_directory(&candidate.root)?;
    unchanged(candidate)?;
    // Strictly reload the persisted scope before giving it replacement authority.
    Intent::load(&operation, &candidate.root)?;
    storage.select(&operation)?;
    if let Err(error) = transaction::commit(&operation, &mut intent) {
        return Err(incomplete(&candidate.root, approved, error));
    }
    let installed = state::inspect(&candidate.root, &candidate.artifact.target)
        .map_err(|error| incomplete(&candidate.root, approved, error))?
        .ok_or(ArtifactError::Invalid("installed receipt is missing"))?;
    Ok(
        serde_json::json!({"schema_version":"rullst.cli-installation-result.v1", "root":candidate.root,
        "operation":operation, "artifact":installed.manifest, "installed":true, "recovered":false,
        "approved_review":approved, "candidate_version_checks_passed":true,
        "known_predecessor_retained":candidate.prior.is_some(), "project_changed":false,"deployment_performed":false,
        "path_changed":false,"recovery_args":["update","install","recover","--root",candidate.root,"--approved-review",approved]}),
    )
}

pub(super) fn recover(root: &Path, approved: &str) -> Result<serde_json::Value, ArtifactError> {
    recover_with(root, approved, authenticate)
}

pub(super) fn recover_with(
    root: &Path,
    approved: &str,
    mut verify: impl FnMut(&[u8], &super::Manifest) -> Result<(), ArtifactError>,
) -> Result<serde_json::Value, ArtifactError> {
    let storage = Storage::open(root)?;
    let operation = storage.current()?.ok_or(ArtifactError::Invalid(
        "no selected installation operation to recover",
    ))?;
    let mut intent = Intent::load(&operation, root)?;
    if intent.approved_review != approved {
        return Err(ArtifactError::Invalid(
            "recovery approval differs from the selected installation review",
        ));
    }
    let target = super::super::native_target()?;
    let after = transaction::parse_manifest(intent.manifest.as_bytes(), target)?;
    verify(intent.manifest.as_bytes(), &after)?;
    if let Some(raw) = &intent.prior_manifest {
        let before = transaction::parse_manifest(raw.as_bytes(), target)?;
        verify(raw.as_bytes(), &before)?;
    }
    let already = intent.phase == Phase::Recovered;
    recovery::restore(&operation, &mut intent)
        .map_err(|error| incomplete(root, approved, error))?;
    Ok(
        serde_json::json!({"schema_version":"rullst.cli-installation-result.v1", "root":root,"operation":operation,
        "installed":false,"recovered":true,"already_recovered":already,"approved_review":approved,
        "predecessor_restored":intent.prior_manifest.is_some(),"project_changed":false,"deployment_performed":false,"path_changed":false}),
    )
}

fn incomplete(root: &Path, approved: &str, source: ArtifactError) -> ArtifactError {
    ArtifactError::Installation {
        root: root.to_owned(),
        approved: approved.into(),
        source: Box::new(source),
    }
}
