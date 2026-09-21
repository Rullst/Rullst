use rullst_labs::{ContentHash, ExecutionProfile, LabError as Error};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LinuxConfig {
    pub rootfs: PathBuf,
    pub launcher: PathBuf,
    pub cgroups: PathBuf,
    pub profile: ExecutionProfile,
}
impl LinuxConfig {
    pub fn validate(&self) -> Result<(), Error> {
        let ExecutionProfile::LinuxExperimental { tools, .. } = &self.profile else {
            return Err(Error::Unsupported);
        };
        for path in [&self.rootfs, &self.launcher, &self.cgroups] {
            if !path.is_absolute()
                || path.canonicalize().map_err(|_| Error::Configuration)? != *path
            {
                return Err(Error::Configuration);
            }
        }
        trusted_directory(&self.rootfs)?;
        if hash_file(&std::env::current_exe().map_err(|_| Error::Configuration)?)? != tools.runner
            || hash_file(&self.rootfs.join("runner"))? != tools.runner
            || hash_file(&self.rootfs.join("toolchain/bin/rustc"))? != tools.compiler
            || hash_tree(&self.rootfs.join("toolchain"))? != tools.wasm_toolchain
            || hash_tree(&self.rootfs.join("runtime"))? != tools.runtime
            || hash_file(&self.launcher)? != tools.launcher
            || super::syscalls::fingerprint()? != tools.syscall_policy
            || super::filesystem::fingerprint()? != tools.filesystem_policy
        {
            return Err(Error::Integrity);
        }
        super::cgroup::validate_root(&self.cgroups)
    }
}
pub(super) fn hash_file(path: &Path) -> Result<ContentHash, Error> {
    trusted_ancestors(path)?;
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Configuration)?;
    if !trusted_owner(&meta)
        || !meta.is_file()
        || meta.mode() & 0o6022 != 0
        || meta.len() > 536_870_912
    {
        return Err(Error::Configuration);
    }
    let mut file = std::fs::File::open(path).map_err(|_| Error::Configuration)?;
    let mut digest = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buffer = [0u8; 65536];
    let mut total = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|_| Error::Configuration)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or(Error::Capacity)?;
        if total > meta.len() {
            return Err(Error::Integrity);
        }
        digest.update(&buffer[..count]);
    }
    if total != meta.len() {
        return Err(Error::Integrity);
    }
    ContentHash::new(hex::encode(digest.finish().as_ref()))
}
pub(super) fn hash_tree(root: &Path) -> Result<ContentHash, Error> {
    trusted_directory(root)?;
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut count = 0usize;
    let mut total = 0u64;
    while let Some(path) = pending.pop() {
        count += 1;
        if count > 4096 {
            return Err(Error::Capacity);
        }
        let meta = std::fs::symlink_metadata(&path).map_err(|_| Error::Configuration)?;
        if !trusted_owner(&meta) || meta.mode() & 0o6022 != 0 {
            return Err(Error::Configuration);
        }
        if meta.is_dir() {
            for child in std::fs::read_dir(&path).map_err(|_| Error::Configuration)? {
                pending.push(child.map_err(|_| Error::Configuration)?.path());
                if pending.len() > 4096 {
                    return Err(Error::Capacity);
                }
            }
        } else if meta.is_file() {
            total = total.checked_add(meta.len()).ok_or(Error::Capacity)?;
            if total > 2_147_483_648 {
                return Err(Error::Capacity);
            }
            let name = path
                .strip_prefix(root)
                .map_err(|_| Error::Configuration)?
                .to_str()
                .ok_or(Error::Configuration)?
                .to_owned();
            if name.len() > 512 {
                return Err(Error::Configuration);
            }
            files.push((name, hash_file(&path)?));
        } else {
            return Err(Error::Configuration);
        }
    }
    if files.is_empty() {
        return Err(Error::Configuration);
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(ContentHash::of(
        &serde_json::to_vec(&("RullstLabsToolTree-v1", files)).map_err(|_| Error::Configuration)?,
    ))
}

fn trusted_owner(meta: &std::fs::Metadata) -> bool {
    meta.uid() == 0 || meta.uid() == rustix::process::getuid().as_raw()
}

fn trusted_directory(path: &Path) -> Result<(), Error> {
    trusted_ancestors(path)?;
    let meta = std::fs::symlink_metadata(path).map_err(|_| Error::Configuration)?;
    if !meta.is_dir() || !trusted_owner(&meta) || meta.mode() & 0o6022 != 0 {
        return Err(Error::Configuration);
    }
    Ok(())
}

/// Files are not immutable to other users if they can replace an ancestor.
/// Sticky shared ancestors such as /tmp are allowed only with trusted ownership:
/// the next owned component cannot be renamed/unlinked by another local UID.
fn trusted_ancestors(path: &Path) -> Result<(), Error> {
    if !path.is_absolute() || path.canonicalize().map_err(|_| Error::Configuration)? != path {
        return Err(Error::Configuration);
    }
    for parent in path.ancestors().skip(1) {
        let meta = std::fs::symlink_metadata(parent).map_err(|_| Error::Configuration)?;
        if !meta.is_dir()
            || !trusted_owner(&meta)
            || (meta.mode() & 0o0022 != 0 && meta.mode() & 0o1000 == 0)
        {
            return Err(Error::Configuration);
        }
    }
    Ok(())
}
