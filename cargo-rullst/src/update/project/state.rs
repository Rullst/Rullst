use super::{Prepared, ProjectError, snapshot};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub(super) struct State {
    stage: PathBuf,
    record_digest: String,
    pub prepared: Prepared,
    pub paths: BTreeSet<String>,
    pub candidate: PathBuf,
    pub candidate_records: Vec<snapshot::Record>,
}

impl State {
    pub fn load(stage: &Path) -> Result<Self, ProjectError> {
        let bytes = snapshot::read(stage, "preparation.json")?
            .ok_or(ProjectError::Invalid("missing preparation record"))?;
        let prepared: Prepared = serde_json::from_slice(&bytes)?;
        if prepared.schema_version != "rullst.project-preparation.v1"
            || prepared.phase != "prepared"
            || prepared.platform != std::env::consts::OS
            || prepared.execution_authorized
            || prepared.application_authorized
            || prepared.files.len() > snapshot::MAX_ENTRIES
            || prepared.files.is_empty()
            || prepared.source.canonicalize()? != prepared.source
        {
            return Err(ProjectError::Invalid(
                "invalid or foreign-platform preparation record",
            ));
        }
        let mut paths = BTreeSet::new();
        let mut present = BTreeSet::new();
        let mut total = 0u64;
        let before = stage.join("before");
        let candidate = stage.join("candidate");
        for file in &prepared.files {
            total = total
                .checked_add(file.bytes)
                .ok_or(ProjectError::Invalid("preparation size overflow"))?;
            if total > snapshot::MAX_TOTAL {
                return Err(ProjectError::Invalid(
                    "preparation exceeds its total byte bound",
                ));
            }
            snapshot::validate_path(&file.path)?;
            if !paths.insert(file.path.clone()) {
                return Err(ProjectError::Invalid("duplicate preparation input"));
            }
            if file.sha256.is_some() {
                present.insert(file.path.clone());
            }
            let body = snapshot::read(&before, &file.path)?;
            if snapshot::record(&file.path, body.as_deref()) != *file {
                return Err(ProjectError::Invalid(
                    "baseline bytes differ from preparation record",
                ));
            }
        }
        if !paths.contains("Cargo.toml")
            || !paths.contains("Cargo.lock")
            || snapshot::tree(&before)? != present
            || snapshot::tree(&candidate)? != present
        {
            return Err(ProjectError::Invalid("prepared file inventory changed"));
        }
        snapshot::unchanged(&prepared.source, &paths, &prepared.files)?;
        // Cargo may return C:\... while private storage uses \?\C:\... .
        // Compare filesystem-resolved paths, not incompatible lexical prefixes.
        // The complete source tree above has already rejected links/reparse points.
        let canonical_candidate = candidate.canonicalize()?;
        let manifests: Vec<_> = super::manifests(&candidate)?
            .into_iter()
            .map(|path| {
                let path = path.canonicalize()?;
                let relative = path
                    .strip_prefix(&canonical_candidate)
                    .map_err(|_| ProjectError::Invalid("workspace member leaves preparation"))?
                    .to_str()
                    .ok_or(ProjectError::Invalid("manifest path is not UTF-8"))?
                    .replace('\\', "/");
                snapshot::validate_path(&relative)?;
                if !present.contains(&relative) {
                    return Err(ProjectError::Invalid("manifest is not a prepared input"));
                }
                Ok(relative)
            })
            .collect::<Result<_, ProjectError>>()?;
        let manifest_set: BTreeSet<_> = manifests.iter().map(String::as_str).collect();
        let plan = crate::generators::build::validate_prepared_manifests(
            &before,
            &candidate,
            &manifests,
            &prepared.target,
        )
        .map_err(|error| ProjectError::Planning(error.to_string()))?;
        if plan != prepared.plan {
            return Err(ProjectError::Invalid(
                "preparation plan does not match the current migration catalog",
            ));
        }
        let findings = plan["source_findings"]
            .as_array()
            .ok_or(ProjectError::Invalid("missing source review findings"))?;
        if !findings.is_empty() {
            return Err(ProjectError::Invalid(
                "resolve migration findings in the original project and prepare again before execution",
            ));
        }
        let mut candidate_records = Vec::new();
        for file in &prepared.files {
            let body = snapshot::read(&candidate, &file.path)?;
            let record = snapshot::record(&file.path, body.as_deref());
            if !manifest_set.contains(file.path.as_str()) && record != *file {
                return Err(ProjectError::Invalid(
                    "candidate source changed; review the original and prepare again",
                ));
            }
            candidate_records.push(record);
        }
        Ok(Self {
            stage: stage.into(),
            record_digest: hex::encode(Sha256::digest(&bytes)),
            prepared,
            paths,
            candidate,
            candidate_records,
        })
    }

    pub fn unchanged(&self) -> Result<(), ProjectError> {
        let record = snapshot::read(&self.stage, "preparation.json")?
            .ok_or(ProjectError::Invalid("preparation record disappeared"))?;
        if hex::encode(Sha256::digest(&record)) != self.record_digest {
            return Err(ProjectError::Invalid(
                "preparation record changed during verification",
            ));
        }
        let baseline = self.stage.join("before");
        let present: BTreeSet<_> = self
            .prepared
            .files
            .iter()
            .filter(|file| file.sha256.is_some())
            .map(|file| file.path.clone())
            .collect();
        if snapshot::tree(&baseline)? != present {
            return Err(ProjectError::Invalid(
                "baseline inventory changed during verification",
            ));
        }
        for file in &self.prepared.files {
            let bytes = snapshot::read(&baseline, &file.path)?;
            if snapshot::record(&file.path, bytes.as_deref()) != *file {
                return Err(ProjectError::Invalid(
                    "baseline changed during verification",
                ));
            }
        }
        snapshot::unchanged(&self.prepared.source, &self.paths, &self.prepared.files)?;
        let expected: BTreeSet<_> = self
            .candidate_records
            .iter()
            .filter(|file| file.sha256.is_some())
            .map(|file| file.path.clone())
            .collect();
        if snapshot::tree(&self.candidate)? != expected {
            return Err(ProjectError::Invalid(
                "candidate inventory changed during verification",
            ));
        }
        for file in &self.candidate_records {
            let body = snapshot::read(&self.candidate, &file.path)?;
            if snapshot::record(&file.path, body.as_deref()) != *file {
                return Err(ProjectError::Invalid(
                    "candidate changed during verification",
                ));
            }
        }
        Ok(())
    }
}
