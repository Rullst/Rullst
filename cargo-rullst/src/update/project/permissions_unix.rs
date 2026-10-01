use super::{Permissions, ProjectError, fs};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};

pub(super) fn capture(path: Option<&Path>) -> Result<Permissions, ProjectError> {
    let (mode, uid, gid) = if let Some(path) = path {
        let file = fs::File::open(path)?;
        plain(&file)?;
        let metadata = file.metadata()?;
        if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o7000 != 0 {
            return Err(ProjectError::Invalid(
                "replacement requires a caller-owned file without special mode bits",
            ));
        }
        (metadata.mode() & 0o777, metadata.uid(), metadata.gid())
    } else {
        (
            0o600,
            rustix::process::geteuid().as_raw(),
            rustix::process::getegid().as_raw(),
        )
    };
    Ok(Permissions {
        readonly: mode & 0o222 == 0,
        unix_mode: Some(mode),
        unix_owner: Some((uid, gid)),
        windows_descriptor: None,
    })
}

pub(super) fn stage(
    policy: &Permissions,
    parent: &Path,
) -> Result<tempfile::NamedTempFile, ProjectError> {
    let mode = policy
        .unix_mode
        .filter(|mode| mode & !0o777 == 0)
        .ok_or(ProjectError::Invalid("invalid saved file permissions"))?;
    let (uid, gid) = policy
        .unix_owner
        .ok_or(ProjectError::Invalid("missing saved owner/group"))?;
    if policy.windows_descriptor.is_some() || uid != rustix::process::geteuid().as_raw() {
        return Err(ProjectError::Invalid("foreign file access policy"));
    }
    let file = tempfile::Builder::new()
        .prefix(".rullst-update-stage-")
        .tempfile_in(parent)?;
    // Inherited ACLs/xattrs are rejected before any candidate bytes are written.
    plain(file.as_file())?;
    rustix::fs::fchown(
        file.as_file(),
        Some(rustix::process::Uid::from_raw(uid)),
        Some(rustix::process::Gid::from_raw(gid)),
    )
    .map_err(std::io::Error::from)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    plain(file.as_file())?;
    Ok(file)
}

#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
const EXTENDED_METADATA: ProjectError = ProjectError::Invalid(
    "extended file metadata requires manual update; no ACL/xattrs are discarded",
);

/// Largest attribute-name list inspected (Linux's `XATTR_LIST_MAX`).
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
const MAX_XATTR_NAMES: usize = 64 * 1024;

fn plain(file: &fs::File) -> Result<(), ProjectError> {
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
    {
        if has_unreproduced_attribute(&xattr_names(file)?) {
            return Err(EXTENDED_METADATA);
        }
        #[cfg(target_os = "macos")]
        apple_acl::check(file)?;
        Ok(())
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "macos")))]
    {
        let _ = file;
        Err(ProjectError::Invalid(
            "extended access-policy inspection is unavailable on this platform",
        ))
    }
}

/// The NUL-separated extended attribute names of `file`.
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
fn xattr_names(file: &fs::File) -> Result<Vec<u8>, ProjectError> {
    // The list can grow between the size query and the read; retry briefly.
    for _ in 0..4 {
        let size = match rustix::fs::flistxattr(file, &mut [0u8; 0][..]) {
            Ok(size) => size,
            // A filesystem with no xattr support cannot carry POSIX ACL xattrs.
            Err(rustix::io::Errno::NOTSUP) => return Ok(Vec::new()),
            Err(error) => return Err(std::io::Error::from(error).into()),
        };
        if size == 0 {
            return Ok(Vec::new());
        }
        if size > MAX_XATTR_NAMES {
            return Err(EXTENDED_METADATA);
        }
        let mut names = vec![0u8; size];
        match rustix::fs::flistxattr(file, &mut names[..]) {
            Ok(length) => {
                names.truncate(length);
                return Ok(names);
            }
            Err(rustix::io::Errno::RANGE) => continue,
            Err(error) => return Err(std::io::Error::from(error).into()),
        }
    }
    Err(EXTENDED_METADATA)
}

/// Whether `names` holds an attribute that replacing the file would drop.
///
/// The SELinux label (`security.selinux`) is the exception: the kernel gives
/// every file one, including the replacement staged in the same directory,
/// so rejecting it made review/apply fail on every SELinux host. ACLs
/// (`system.posix_acl_*`), file capabilities and other attributes still fail.
#[cfg(any(target_os = "linux", target_os = "android", target_os = "macos"))]
fn has_unreproduced_attribute(names: &[u8]) -> bool {
    names
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .any(|name| {
            !(cfg!(any(target_os = "linux", target_os = "android")) && name == b"security.selinux")
        })
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod apple_acl {
    use super::*;
    use std::{ffi::c_void, os::fd::AsRawFd};
    // macOS extended ACLs are separate from the listxattr namespace.
    unsafe extern "C" {
        fn acl_get_fd(fd: i32) -> *mut c_void;
        fn acl_get_entry(acl: *mut c_void, entry_id: i32, entry: *mut *mut c_void) -> i32;
        fn acl_free(object: *mut c_void) -> i32;
    }
    pub(super) fn check(file: &fs::File) -> Result<(), ProjectError> {
        // SAFETY: the borrowed file descriptor remains open throughout; the
        // returned ACL is owned and freed exactly once after entry inspection.
        let acl = unsafe { acl_get_fd(file.as_raw_fd()) };
        if acl.is_null() {
            let error = std::io::Error::last_os_error();
            // Darwin filesec_get_property reports ENOENT for an absent ACL on
            // this already-open fd; no pathname lookup occurs here.
            if error.raw_os_error() == Some(2) {
                return Ok(());
            }
            return Err(error.into());
        }
        if acl.addr() == 1 {
            return Err(ProjectError::Invalid("unexpected Darwin ACL sentinel"));
        }
        let mut entry = std::ptr::null_mut();
        // SAFETY: acl is the live owned acl_get_fd allocation, entry is a valid
        // output pointer, and zero selects its first entry on Darwin. The
        // borrowed entry is never dereferenced or retained after freeing acl.
        let result = unsafe { acl_get_entry(acl, 0, &mut entry) };
        let error = std::io::Error::last_os_error();
        // SAFETY: this is the allocation returned by acl_get_fd, freed once;
        // neither acl nor the borrowed entry is used after this call.
        unsafe {
            acl_free(acl);
        }
        if result == 0 {
            return Err(ProjectError::Invalid("extended ACL requires manual update"));
        }
        // Darwin returns -1/EINVAL when no first entry exists (unlike Linux).
        if error.raw_os_error() != Some(22) {
            return Err(error.into());
        }
        Ok(())
    }
}

#[cfg(all(
    test,
    any(target_os = "linux", target_os = "android", target_os = "macos")
))]
mod tests {
    use super::*;

    #[test]
    fn only_the_selinux_label_is_reproduced_by_a_staged_replacement() {
        assert!(!has_unreproduced_attribute(b""));
        assert_eq!(
            has_unreproduced_attribute(b"security.selinux\0"),
            !cfg!(any(target_os = "linux", target_os = "android"))
        );
        for names in [
            &b"security.selinux\0user.note\0"[..],
            b"system.posix_acl_access\0",
            b"security.capability\0",
            b"user.rullst-test\0",
        ] {
            assert!(has_unreproduced_attribute(names), "{names:?}");
        }
    }
}
