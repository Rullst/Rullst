use super::ReleaseError;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

pub(super) const APK_LIMIT: u64 = 512 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    pub(super) bytes: u64,
    pub(super) digest: String,
    modified: SystemTime,
}

fn metadata(path: &Path) -> Result<fs::Metadata, ReleaseError> {
    let metadata = fs::symlink_metadata(path)?;
    let linked = metadata.is_symlink();
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        linked || metadata.file_attributes() & 0x400 != 0
    };
    if linked || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(ReleaseError::Artifact);
    }
    Ok(metadata)
}

pub(super) fn bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>, ReleaseError> {
    let info = metadata(path)?;
    if !info.is_file() || info.len() == 0 || info.len() > limit {
        return Err(ReleaseError::Artifact);
    }
    let mut body = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut body)?;
    if body.is_empty() || body.len() as u64 > limit {
        return Err(ReleaseError::Artifact);
    }
    Ok(body)
}

pub(super) fn stamp(path: &Path) -> Result<Stamp, ReleaseError> {
    let info = metadata(path)?;
    if !info.is_file() || info.len() == 0 || info.len() > APK_LIMIT {
        return Err(ReleaseError::Artifact);
    }
    let mut reader = fs::File::open(path)?.take(APK_LIMIT + 1);
    let mut hash = Sha256::new();
    let mut bytes = 0;
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        if bytes > APK_LIMIT {
            return Err(ReleaseError::Artifact);
        }
        hash.update(&buffer[..read]);
    }
    if bytes != info.len() {
        return Err(ReleaseError::Artifact);
    }
    Ok(Stamp {
        bytes,
        digest: hex::encode(hash.finalize()),
        modified: info.modified()?,
    })
}

pub(super) fn relative(path: &Path) -> Result<(), ReleaseError> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(ReleaseError::Artifact);
    }
    if !path
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.ends_with("-release.apk"))
    {
        return Err(ReleaseError::Artifact);
    }
    Ok(())
}

pub(super) fn inventory(root: &Path) -> Result<BTreeMap<PathBuf, Stamp>, ReleaseError> {
    let mut result = BTreeMap::new();
    // Validate each generated descendant even before the outputs exist. The
    // caller supplies the canonical application-owned omni-app root.
    let mut output = root.to_path_buf();
    for part in ["gen", "android", "app", "build", "outputs", "apk"] {
        output.push(part);
        match fs::symlink_metadata(&output) {
            Ok(_) if metadata(&output)?.is_dir() => (),
            Ok(_) => return Err(ReleaseError::Artifact),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(result),
            Err(error) => return Err(error.into()),
        }
    }
    let mut count = 0;
    let mut total = 0_u64;
    for entry in walkdir::WalkDir::new(&output)
        .follow_links(false)
        .max_depth(8)
    {
        let entry = entry.map_err(|_| ReleaseError::Artifact)?;
        count += 1;
        if count > 4096 {
            return Err(ReleaseError::Artifact);
        }
        let info = metadata(entry.path())?;
        if info.is_dir() {
            if entry.depth() == 8 {
                return Err(ReleaseError::Artifact);
            }
            continue;
        }
        let relative_path = entry
            .path()
            .strip_prefix(&output)
            .map_err(|_| ReleaseError::Artifact)?;
        if relative(relative_path).is_err() {
            continue;
        }
        total = total
            .checked_add(info.len())
            .ok_or(ReleaseError::Artifact)?;
        if total > APK_LIMIT || result.len() >= 16 {
            return Err(ReleaseError::Artifact);
        }
        result.insert(relative_path.to_owned(), stamp(entry.path())?);
    }
    Ok(result)
}

pub(super) fn select(
    before: &BTreeMap<PathBuf, Stamp>,
    after: &BTreeMap<PathBuf, Stamp>,
    selected: Option<&Path>,
    start: SystemTime,
) -> Result<PathBuf, ReleaseError> {
    let fresh: Vec<_> = after
        .iter()
        .filter(|(path, value)| {
            selected.is_none_or(|chosen| chosen == path.as_path())
                && before.get(*path) != Some(*value)
                && value.modified >= start
        })
        .map(|(path, _)| path)
        .collect();
    if fresh.len() != 1 {
        return Err(ReleaseError::Selection);
    }
    fresh
        .first()
        .map(|p| (*p).clone())
        .ok_or(ReleaseError::Selection)
}

pub(super) fn snapshot(
    path: &Path,
    expected: &Stamp,
    destination: &Path,
) -> Result<(), ReleaseError> {
    let mut reader = fs::File::open(path)?.take(APK_LIMIT + 1);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let bytes = std::io::copy(&mut reader, &mut file)?;
    file.flush()?;
    if bytes != expected.bytes || stamp(destination)?.digest != expected.digest {
        return Err(ReleaseError::Artifact);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(modified: SystemTime) -> Stamp {
        Stamp {
            bytes: 1,
            digest: "00".repeat(32),
            modified,
        }
    }

    #[test]
    fn selection_requires_changed_output_from_this_build_and_an_unambiguous_variant() {
        let start = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(100);
        let a = PathBuf::from("arm64/release/app-release.apk");
        let b = PathBuf::from("universal/release/app-release.apk");
        let before = BTreeMap::from([(a.clone(), record(start))]);
        assert!(select(&before, &before, None, start).is_err());
        let fresh =
            BTreeMap::from([(a.clone(), record(start + std::time::Duration::from_secs(1)))]);
        assert_eq!(select(&before, &fresh, None, start).unwrap(), a);
        let old = BTreeMap::from([(a.clone(), record(start - std::time::Duration::from_secs(1)))]);
        assert!(select(&BTreeMap::new(), &old, None, start).is_err());
        let mut ambiguous = fresh;
        ambiguous.insert(b, record(start));
        assert!(select(&before, &ambiguous, None, start).is_err());
        assert_eq!(select(&before, &ambiguous, Some(&a), start).unwrap(), a);
        assert!(
            select(
                &before,
                &ambiguous,
                Some(Path::new("missing-release.apk")),
                start
            )
            .is_err()
        );
    }

    #[test]
    fn outside_unsigned_empty_and_oversized_artifacts_are_rejected() {
        for invalid in [
            "",
            "../app-release.apk",
            "/app-release.apk",
            "app-unsigned.apk",
            "app.apk",
        ] {
            assert!(relative(Path::new(invalid)).is_err());
        }
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("app-release.apk");
        fs::write(&file, "").unwrap();
        assert!(stamp(&file).is_err());
        fs::File::create(&file)
            .unwrap()
            .set_len(APK_LIMIT + 1)
            .unwrap();
        assert!(stamp(&file).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn linked_artifacts_and_output_parents_never_enter_the_inventory() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("gen")).unwrap();
        assert!(inventory(root.path()).is_err());
        fs::remove_file(root.path().join("gen")).unwrap();
        let output = root.path().join("gen/android/app/build/outputs/apk");
        fs::create_dir_all(&output).unwrap();
        let secret = outside.path().join("user-file");
        fs::write(&secret, "private").unwrap();
        std::os::unix::fs::symlink(&secret, output.join("app-release.apk")).unwrap();
        assert!(inventory(root.path()).is_err());
        assert_eq!(fs::read_to_string(secret).unwrap(), "private");
    }
}
