use super::{ProjectError, process};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, Metadata, OpenOptions},
    io::{Read, Write},
    path::{Component, Path},
};

pub(super) const MAX_ENTRIES: usize = 100_000;
const MAX_FILE: u64 = 64 * 1024 * 1024;
pub(super) const MAX_TOTAL: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub path: String,
    pub bytes: u64,
    pub sha256: Option<String>,
}

pub(super) fn inventory(root: &Path) -> Result<BTreeSet<String>, ProjectError> {
    let bytes = process::capture(
        "git",
        &[
            "--literal-pathspecs",
            "-c",
            "core.fsmonitor=false",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            ".",
        ],
        root,
        16 * 1024 * 1024,
    )?;
    let mut paths = BTreeSet::new();
    for name in bytes
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
    {
        let text = std::str::from_utf8(name)
            .map_err(|_| ProjectError::Invalid("project filenames must be valid UTF-8"))?;
        validate_path(text)?;
        if !paths.insert(text.to_owned()) {
            return Err(ProjectError::Invalid(
                "resolve Git index conflicts before preparation",
            ));
        }
        if paths.len() > MAX_ENTRIES {
            return Err(ProjectError::Invalid(
                "project inventory exceeds 100,000 entries",
            ));
        }
    }
    if !paths.contains("Cargo.toml") {
        return Err(ProjectError::Invalid(
            "select a Git project directory containing Cargo.toml",
        ));
    }
    // The legacy generator ignored Cargo.lock. Retain this reproducibility
    // input explicitly, including its absence, without copying ignored secrets.
    paths.insert("Cargo.lock".into());
    if paths.len() > MAX_ENTRIES {
        return Err(ProjectError::Invalid(
            "project inventory exceeds 100,000 entries",
        ));
    }
    Ok(paths)
}

pub(super) fn validate_path(text: &str) -> Result<(), ProjectError> {
    if text.is_empty()
        || text.len() > 4096
        || text.contains(['\\', ':'])
        || text.chars().any(char::is_control)
        || text.split('/').any(|part| {
            part.is_empty() || matches!(part, "." | ".." | ".git") || part.ends_with([' ', '.'])
        })
        || Path::new(text)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(ProjectError::Invalid(
            "project inventory contains an unsupported or unsafe relative path",
        ));
    }
    if text.split('/').any(|part| part == "target") {
        return Err(ProjectError::Invalid(
            "remove generated target artifacts from the Git input inventory before preparation",
        ));
    }
    Ok(())
}

fn linked(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes()
            & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
            != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(super) fn read(root: &Path, name: &str) -> Result<Option<Vec<u8>>, ProjectError> {
    validate_path(name)?;
    let relative = Path::new(name);
    let mut parent = root.to_path_buf();
    if let Some(path) = relative.parent() {
        for component in path.components() {
            parent.push(component);
            let metadata = match fs::symlink_metadata(&parent) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(error.into()),
            };
            if !metadata.is_dir() || linked(&metadata) {
                return Err(ProjectError::Invalid(
                    "project input has a linked or non-directory ancestor",
                ));
            }
        }
    }
    let path = root.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || linked(&metadata) || metadata.len() > MAX_FILE {
        return Err(ProjectError::Invalid(
            "project inputs must be regular files bounded to 64 MiB; links/submodules/special files are unsupported",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(
            (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let opened = file.metadata()?;
    if !opened.is_file() || linked(&opened) || opened.len() != metadata.len() {
        return Err(ProjectError::Invalid("project input changed while opening"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > MAX_FILE {
        return Err(ProjectError::Invalid(
            "project input changed size while copying",
        ));
    }
    Ok(Some(bytes))
}

pub(super) fn record(name: &str, body: Option<&[u8]>) -> Record {
    Record {
        path: name.to_owned(),
        bytes: body.map_or(0, |value| value.len() as u64),
        sha256: body.map(|value| hex::encode(Sha256::digest(value))),
    }
}

pub(super) fn tree(root: &Path) -> Result<BTreeSet<String>, ProjectError> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || linked(&metadata) {
        return Err(ProjectError::Invalid(
            "prepared source root must be an unlinked directory",
        ));
    }
    let mut paths = BTreeSet::new();
    let mut count = 0;
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|_| ProjectError::Invalid("cannot enumerate prepared source"))?;
        count += 1;
        if count > 2 * MAX_ENTRIES {
            return Err(ProjectError::Invalid(
                "prepared source entry count exceeds its bound",
            ));
        }
        if entry.depth() == 0 {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| ProjectError::Invalid("source escapes preparation"))?
            .to_str()
            .ok_or(ProjectError::Invalid("prepared paths must be UTF-8"))?
            .replace('\\', "/");
        validate_path(&relative)?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if linked(&metadata) || (!metadata.is_dir() && !metadata.is_file()) {
            return Err(ProjectError::Invalid(
                "prepared source contains linked or special inputs",
            ));
        }
        if metadata.is_file() {
            paths.insert(relative);
        }
        if paths.len() > MAX_ENTRIES {
            return Err(ProjectError::Invalid(
                "prepared source exceeds its file count limit",
            ));
        }
    }
    Ok(paths)
}

fn write(root: &Path, name: &str, body: &[u8]) -> Result<(), ProjectError> {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(body)?;
    Ok(())
}

pub(super) fn copy(
    root: &Path,
    baseline: &Path,
    candidate: &Path,
    paths: &BTreeSet<String>,
) -> Result<Vec<Record>, ProjectError> {
    fs::create_dir(baseline)?;
    fs::create_dir(candidate)?;
    let mut records = Vec::new();
    let mut total = 0u64;
    for name in paths {
        let body = read(root, name)?;
        let entry = record(name, body.as_deref());
        total = total
            .checked_add(entry.bytes)
            .ok_or(ProjectError::Invalid("project size overflow"))?;
        if total > MAX_TOTAL {
            return Err(ProjectError::Invalid(
                "project source snapshot exceeds 512 MiB",
            ));
        }
        if let Some(body) = body {
            write(baseline, name, &body)?;
            write(candidate, name, &body)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let executable = fs::metadata(root.join(name))?.permissions().mode() & 0o100 != 0;
                if executable {
                    fs::set_permissions(baseline.join(name), fs::Permissions::from_mode(0o700))?;
                    fs::set_permissions(candidate.join(name), fs::Permissions::from_mode(0o700))?;
                }
            }
        }
        records.push(entry);
    }
    Ok(records)
}

pub(super) fn unchanged(
    root: &Path,
    paths: &BTreeSet<String>,
    records: &[Record],
) -> Result<(), ProjectError> {
    unchanged_ignoring(root, paths, records, &BTreeSet::new())
}

pub(super) fn unchanged_ignoring(
    root: &Path,
    paths: &BTreeSet<String>,
    records: &[Record],
    owned_staging: &BTreeSet<String>,
) -> Result<(), ProjectError> {
    let mut current = inventory(root)?;
    for name in owned_staging {
        current.remove(name);
    }
    if current != *paths {
        return Err(ProjectError::Invalid(
            "project inventory changed during preparation; retry with writers stopped",
        ));
    }
    for entry in records {
        let body = read(root, &entry.path)?;
        if record(&entry.path, body.as_deref()) != *entry {
            return Err(ProjectError::Invalid(
                "project contents changed during preparation; retry with writers stopped",
            ));
        }
    }
    Ok(())
}
