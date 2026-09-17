use super::ArtifactError;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::Read,
    path::Path,
};

pub(super) fn directory(path: &Path) -> Result<(), ArtifactError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || linked(&metadata) {
        return Err(ArtifactError::Invalid(
            "artifact directory must not be a link or reparse point",
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

fn open(path: &Path, limit: u64) -> Result<File, ArtifactError> {
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
    let metadata = file.metadata()?;
    if !metadata.is_file() || linked(&metadata) || metadata.len() == 0 || metadata.len() > limit {
        return Err(ArtifactError::Invalid(
            "artifact must be a bounded nonempty regular file",
        ));
    }
    Ok(file)
}

pub(super) fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, ArtifactError> {
    let file = open(path, limit)?;
    let expected = file.metadata()?.len();
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != expected || bytes.len() as u64 > limit {
        return Err(ArtifactError::Invalid(
            "artifact changed size while reading",
        ));
    }
    Ok(bytes)
}

pub(super) fn digest(path: &Path, size: u64) -> Result<String, ArtifactError> {
    let file = open(path, super::manifest::MAX_BINARY)?;
    if file.metadata()?.len() != size {
        return Err(ArtifactError::Invalid(
            "artifact size does not match the manifest",
        ));
    }
    let mut reader = file.take(size + 1);
    let mut hasher = Sha256::new();
    let mut count = 0;
    let mut buffer = [0; 64 * 1024];
    loop {
        let length = reader.read(&mut buffer)?;
        if length == 0 {
            break;
        }
        count += length as u64;
        hasher.update(&buffer[..length]);
    }
    if count != size {
        return Err(ArtifactError::Invalid(
            "artifact changed size while hashing",
        ));
    }
    Ok(hex::encode(hasher.finalize()))
}
