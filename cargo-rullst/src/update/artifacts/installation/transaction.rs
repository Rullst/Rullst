//! A persisted three-entry transaction; each rename is atomic, the set is not.
use super::{
    ArtifactError, Manifest, files, state,
    storage::{self, Storage},
};
use crate::update::cache;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub bytes: u64,
    pub sha256: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Change {
    pub name: String,
    pub before: Option<Record>,
    pub after: Record,
}

#[derive(PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Prepared,
    Complete,
    Recovering,
    Recovered,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub schema: String,
    pub root: PathBuf,
    pub approved_review: String,
    pub phase: Phase,
    pub manifest: String,
    pub prior_manifest: Option<String>,
    pub changes: Vec<Change>,
}

pub(super) fn record(path: &Path) -> Result<Option<Record>, ArtifactError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            cache::installation_file(path)?;
            if metadata.permissions().readonly() {
                return Err(ArtifactError::Invalid(
                    "read-only installed files require manual review",
                ));
            }
            Ok(Some(Record {
                bytes: metadata.len(),
                sha256: files::digest(path, metadata.len())?,
            }))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn matching(path: &Path, expected: &Option<Record>) -> Result<(), ArtifactError> {
    if record(path)? != *expected {
        return Err(ArtifactError::Invalid(
            "installation file changed; conflicting content was not overwritten",
        ));
    }
    Ok(())
}

impl Intent {
    pub fn save(&self, operation: &Path) -> Result<(), ArtifactError> {
        storage::atomic_write(operation, "intent.json", &serde_json::to_vec(self)?)
    }

    pub fn load(operation: &Path, root: &Path) -> Result<Self, ArtifactError> {
        cache::installation_file(&operation.join("intent.json"))?;
        let intent: Self = serde_json::from_slice(&files::read_bounded(
            &operation.join("intent.json"),
            128 * 1024,
        )?)?;
        let target = super::super::native_target()?;
        let names = state::executable_names(target);
        if intent.schema != "rullst.cli-installation-intent.v1"
            || intent.root != root
            || !valid_digest(&intent.approved_review)
            || intent.changes.len() != 3
        {
            return Err(ArtifactError::Invalid("invalid installation intent"));
        }
        for (change, name) in
            intent
                .changes
                .iter()
                .zip([names[0].as_str(), &names[1], state::RECEIPT])
        {
            if change.name != name {
                return Err(ArtifactError::Invalid(
                    "installation intent has an unexpected path",
                ));
            }
            for record in change.before.iter().chain(std::iter::once(&change.after)) {
                if record.bytes == 0
                    || record.bytes > super::super::manifest::MAX_BINARY
                    || !valid_digest(&record.sha256)
                {
                    return Err(ArtifactError::Invalid("invalid installation file record"));
                }
            }
        }
        // Journals cannot expand their write authority beyond the attested manifests.
        let after = parse_manifest(intent.manifest.as_bytes(), target)?;
        let before = intent
            .prior_manifest
            .as_ref()
            .map(|raw| parse_manifest(raw.as_bytes(), target))
            .transpose()?;
        for (index, change) in intent.changes.iter().enumerate() {
            let expected = if index < 2 {
                Record {
                    bytes: after.files[index].bytes,
                    sha256: after.files[index].sha256.clone(),
                }
            } else {
                body_record(&state::receipt(root, intent.manifest.as_bytes())?)
            };
            if change.after != expected {
                return Err(ArtifactError::Invalid(
                    "journal after-state differs from manifest",
                ));
            }
            if index < 2 {
                let expected = before.as_ref().map(|manifest| Record {
                    bytes: manifest.files[index].bytes,
                    sha256: manifest.files[index].sha256.clone(),
                });
                if change.before != expected {
                    return Err(ArtifactError::Invalid(
                        "journal before-state differs from manifest",
                    ));
                }
            } else if change.before.is_some() != before.is_some() {
                return Err(ArtifactError::Invalid(
                    "journal receipt presence is inconsistent",
                ));
            }
        }
        Ok(intent)
    }
}

pub(super) fn parse_manifest(body: &[u8], target: &str) -> Result<Manifest, ArtifactError> {
    if body.len() > 16 * 1024 {
        return Err(ArtifactError::Invalid("manifest exceeds 16 KiB"));
    }
    let value: serde_json::Value = serde_json::from_slice(body)?;
    let version = value["version"]
        .as_str()
        .and_then(|v| semver::Version::parse(v).ok())
        .ok_or(ArtifactError::Invalid(
            "invalid installation manifest version",
        ))?;
    Manifest::parse(body, &version, target)
}

pub(super) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn body_record(body: &[u8]) -> Record {
    Record {
        bytes: body.len() as u64,
        sha256: hex::encode(Sha256::digest(body)),
    }
}

pub(super) fn prepare(
    storage: &Storage,
    root: &Path,
    directory: &Path,
    manifest: &Manifest,
    raw_manifest: &[u8],
    prior: Option<&state::Installed>,
    approved: &str,
) -> Result<(PathBuf, Intent), ArtifactError> {
    if let Some(current) = storage.current()?
        && matches!(
            Intent::load(&current, root)?.phase,
            Phase::Prepared | Phase::Recovering
        )
    {
        return Err(ArtifactError::Invalid(
            "unfinished installation; recover the current operation first",
        ));
    }
    let operation = storage.operation()?;
    let before = cache::create_installation_directory(&operation.join("before"))?;
    let after = cache::create_installation_directory(&operation.join("after"))?;
    cache::create_installation_directory(&operation.join("retired"))?;
    let receipt = state::receipt(root, raw_manifest)?;
    let mut changes = Vec::new();
    for (file, name) in manifest
        .files
        .iter()
        .zip(state::executable_names(&manifest.target))
    {
        let old = record(&root.join(&name))?;
        if let Some(old) = &old {
            storage::copy(
                &root.join(&name),
                &before.join(&name),
                old.bytes,
                &old.sha256,
                true,
            )?;
        }
        storage::copy(
            &directory.join(&file.name),
            &after.join(&name),
            file.bytes,
            &file.sha256,
            true,
        )?;
        changes.push(Change {
            name,
            before: old,
            after: Record {
                bytes: file.bytes,
                sha256: file.sha256.clone(),
            },
        });
    }
    let old = record(&root.join(state::RECEIPT))?;
    if let Some(old) = &old {
        storage::copy(
            &root.join(state::RECEIPT),
            &before.join(state::RECEIPT),
            old.bytes,
            &old.sha256,
            false,
        )?;
    }
    storage::write_new(&after.join(state::RECEIPT), &receipt)?;
    changes.push(Change {
        name: state::RECEIPT.into(),
        before: old,
        after: body_record(&receipt),
    });
    let intent = Intent {
        schema: "rullst.cli-installation-intent.v1".into(),
        root: root.to_owned(),
        approved_review: approved.into(),
        phase: Phase::Prepared,
        manifest: String::from_utf8(raw_manifest.to_vec())
            .map_err(|_| ArtifactError::Invalid("manifest not UTF-8"))?,
        prior_manifest: prior
            .map(|p| String::from_utf8(p.raw_manifest.clone()))
            .transpose()
            .map_err(|_| ArtifactError::Invalid("prior manifest not UTF-8"))?,
        changes,
    };
    intent.save(&operation)?;
    Ok((operation, intent))
}

pub(super) fn commit(operation: &Path, intent: &mut Intent) -> Result<(), ArtifactError> {
    commit_with(operation, intent, |_, _| Ok(()))
}

pub(super) fn commit_with(
    operation: &Path,
    intent: &mut Intent,
    mut observe: impl FnMut(usize, bool) -> Result<(), ArtifactError>,
) -> Result<(), ArtifactError> {
    for change in &intent.changes {
        matching(&intent.root.join(&change.name), &change.before)?;
        matching(
            &operation.join("after").join(&change.name),
            &Some(change.after.clone()),
        )?;
    }
    for (index, change) in intent.changes.iter().enumerate() {
        matching(&intent.root.join(&change.name), &change.before)?;
        if change.before.is_some() {
            // Windows allows moving an in-use image but may not allow unlinking it.
            // Keep the old directory entry until explicit evidence retention cleanup.
            let retired = operation.join("retired").join(&change.name);
            matching(&retired, &None)?;
            fs::rename(intent.root.join(&change.name), retired)?;
            storage::sync(&intent.root)?;
            observe(index, false)?;
        }
        matching(&intent.root.join(&change.name), &None)?;
        fs::rename(
            operation.join("after").join(&change.name),
            intent.root.join(&change.name),
        )?;
        storage::sync(&intent.root)?;
        observe(index, true)?;
    }
    intent.phase = Phase::Complete;
    intent.save(operation)
}
