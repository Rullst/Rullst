use super::{
    ArtifactError, state, storage,
    transaction::{self, Intent, Phase},
};
use crate::update::cache;
use std::{fs, path::Path};

pub(super) fn validate(operation: &Path, intent: &Intent) -> Result<(), ArtifactError> {
    let before = cache::installation_root(&operation.join("before"))?;
    let saved = state::inspect_at(&before, &intent.root, super::super::native_target()?)?;
    if saved.as_ref().map(|value| value.raw_manifest.as_slice())
        != intent.prior_manifest.as_ref().map(|s| s.as_bytes())
    {
        return Err(ArtifactError::Invalid(
            "recovery backup differs from the authenticated predecessor",
        ));
    }
    for change in &intent.changes {
        transaction::matching(&before.join(&change.name), &change.before)?;
        let actual = transaction::record(&intent.root.join(&change.name))?;
        if actual == change.before {
            continue;
        }
        if intent.phase == Phase::Recovered {
            return Err(ArtifactError::Invalid(
                "installation changed after recovery",
            ));
        }
        if actual == Some(change.after.clone()) {
            continue;
        }
        if actual.is_none() && matches!(intent.phase, Phase::Prepared | Phase::Recovering) {
            let retired = transaction::record(&operation.join("retired").join(&change.name))?;
            let removed = transaction::record(&operation.join("removed").join(&change.name))?;
            if (change.before.is_some() && retired == change.before)
                || removed == Some(change.after.clone())
            {
                continue;
            }
        }
        return Err(ArtifactError::Invalid(
            "recovery found unrelated edits; retained evidence was not overwritten",
        ));
    }
    Ok(())
}

pub(super) fn restore(operation: &Path, intent: &mut Intent) -> Result<(), ArtifactError> {
    restore_with(operation, intent, |_, _| Ok(()))
}

pub(super) fn restore_with(
    operation: &Path,
    intent: &mut Intent,
    mut observe: impl FnMut(usize, bool) -> Result<(), ArtifactError>,
) -> Result<(), ArtifactError> {
    validate(operation, intent)?;
    if intent.phase == Phase::Recovered {
        return Ok(());
    }
    let restore = cache::create_installation_directory(&operation.join("restore"))?;
    let removed = cache::create_installation_directory(&operation.join("removed"))?;
    // Stage and verify every old file before moving any current entry.
    for change in &intent.changes {
        if let Some(before) = &change.before {
            let target = restore.join(&change.name);
            match transaction::record(&target)? {
                Some(record) if record == *before => (),
                Some(_) => return Err(ArtifactError::Invalid("recovery staging was changed")),
                None => storage::copy(
                    &operation.join("before").join(&change.name),
                    &target,
                    before.bytes,
                    &before.sha256,
                    change.name != state::RECEIPT,
                )?,
            }
        }
    }
    validate(operation, intent)?;
    intent.phase = Phase::Recovering;
    intent.save(operation)?;
    for (index, change) in intent.changes.iter().enumerate() {
        let target = intent.root.join(&change.name);
        let actual = transaction::record(&target)?;
        if actual == change.before {
            continue;
        }
        if actual == Some(change.after.clone()) {
            let retired = removed.join(&change.name);
            transaction::matching(&retired, &None)?;
            fs::rename(&target, retired)?;
            storage::sync(&intent.root)?;
            observe(index, false)?;
        } else if actual.is_some() {
            return Err(ArtifactError::Invalid(
                "installation changed while recovering",
            ));
        }
        if change.before.is_some() {
            transaction::matching(&target, &None)?;
            transaction::matching(&restore.join(&change.name), &change.before)?;
            fs::rename(restore.join(&change.name), &target)?;
            storage::sync(&intent.root)?;
            observe(index, true)?;
        }
    }
    for change in &intent.changes {
        transaction::matching(&intent.root.join(&change.name), &change.before)?;
    }
    intent.phase = Phase::Recovered;
    intent.save(operation)
}
