//! Win32 OS boundary: owned descriptors, terminated paths and atomic creation.
#![allow(unsafe_code)]
use super::{Permissions, ProjectError, fs};
use std::{
    ffi::c_void,
    os::windows::{
        ffi::OsStrExt,
        fs::MetadataExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
    ptr,
};
use windows_sys::Win32::{
    Foundation::{ERROR_HANDLE_EOF, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{Authorization::*, *},
    Storage::FileSystem::*,
};

const POLICY: OBJECT_SECURITY_INFORMATION = OWNER_SECURITY_INFORMATION
    | GROUP_SECURITY_INFORMATION
    | DACL_SECURITY_INFORMATION
    | LABEL_SECURITY_INFORMATION;
struct Allocation(*mut c_void);
impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY: owns exactly one successful LocalAlloc-family API result.
        unsafe {
            LocalFree(self.0);
        }
    }
}

pub(super) fn capture(path: Option<&Path>) -> Result<Permissions, ProjectError> {
    let (readonly, descriptor) = if let Some(path) = path {
        plain_streams(path)?;
        let file = fs::File::open(path)?;
        let flags = file.metadata()?.file_attributes();
        if flags
            & !(FILE_ATTRIBUTE_NORMAL
                | FILE_ATTRIBUTE_ARCHIVE
                | FILE_ATTRIBUTE_READONLY
                | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED)
            != 0
        {
            return Err(ProjectError::Invalid(
                "special Windows file attributes require manual update",
            ));
        }
        (flags & FILE_ATTRIBUTE_READONLY != 0, describe(&file)?)
    } else {
        (false, default_descriptor()?)
    };
    Ok(Permissions {
        readonly,
        unix_mode: None,
        unix_owner: None,
        windows_descriptor: Some(descriptor),
    })
}

fn default_descriptor() -> Result<String, ProjectError> {
    // A missing root lockfile has no original policy. Canonicalize the private
    // creation policy through an empty OS-created prototype, including the
    // process integrity label, without creating anything in the source tree.
    let workspace = crate::update::cache::project_workspace()?;
    let descriptor = crate::update::cache::private_file_descriptor()?;
    let prototype = tempfile::Builder::new()
        .prefix("policy-")
        .make_in(workspace.path(), |path| create(path, &descriptor))?;
    describe(prototype.as_file())
}

fn describe(file: &fs::File) -> Result<String, ProjectError> {
    let mut descriptor = ptr::null_mut();
    // SAFETY: valid borrowed handle and writable outputs; the returned descriptor
    // is a LocalFree allocation whose interior pointers never escape this scope.
    let result = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            POLICY,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if result != 0 {
        return Err(std::io::Error::from_raw_os_error(result as i32).into());
    }
    let descriptor = Allocation(descriptor);
    reject_resource_policy(file)?;
    let mut text = ptr::null_mut();
    let mut length = 0;
    // SAFETY: descriptor remains alive; Win32 allocates the UTF-16 output string.
    if unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor.0,
            1,
            POLICY,
            &mut text,
            &mut length,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let _text = Allocation(text.cast());
    if text.is_null() || length == 0 || length > 32 * 1024 {
        return Err(ProjectError::Invalid(
            "Windows file policy exceeds 32 Ki UTF-16 units",
        ));
    }
    // Win32 reports buffer capacity, not string length. Inspect only the
    // guaranteed terminated string; trailing allocation bytes may be unused.
    let mut written = 0usize;
    while written < length as usize {
        // SAFETY: within the returned allocation and before the first NUL;
        // successful conversion guarantees these string units are initialized.
        if unsafe { *text.add(written) } == 0 {
            break;
        }
        written += 1;
    }
    if written == length as usize {
        return Err(ProjectError::Invalid("unterminated Windows file policy"));
    }
    // SAFETY: the walk established an initialized UTF-16 prefix in the live allocation.
    let text = unsafe { std::slice::from_raw_parts(text, written) };
    String::from_utf16(text).map_err(|_| ProjectError::Invalid("invalid Windows file policy"))
}

fn reject_resource_policy(file: &fs::File) -> Result<(), ProjectError> {
    let mut descriptor = ptr::null_mut();
    // Resource attributes and central-access policies can restrict access beyond
    // a DACL. These subsets require READ_CONTROL, not audit-log privileges.
    // SAFETY: borrowed live handle, documented flags and writable output pointer.
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            ATTRIBUTE_SECURITY_INFORMATION | SCOPE_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return Err(std::io::Error::from_raw_os_error(status as i32).into());
    }
    let descriptor = Allocation(descriptor);
    let mut present = 0;
    let mut defaulted = 0;
    let mut acl = ptr::null_mut();
    // SAFETY: the descriptor allocation lives through the borrowed SACL inspection.
    if unsafe { GetSecurityDescriptorSacl(descriptor.0, &mut present, &mut acl, &mut defaulted) }
        == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if present != 0 && !acl.is_null() {
        // SAFETY: OS-produced ACL is validated before reading its fixed header.
        if unsafe { IsValidAcl(acl) == 0 || (*acl).AceCount != 0 } {
            return Err(ProjectError::Invalid(
                "Windows resource/central access policy requires manual update",
            ));
        }
    }
    Ok(())
}

pub(super) fn stage(
    policy: &Permissions,
    parent: &Path,
) -> Result<tempfile::NamedTempFile, ProjectError> {
    if policy.readonly || policy.unix_mode.is_some() || policy.unix_owner.is_some() {
        return Err(ProjectError::Invalid(
            "read-only or foreign-platform target requires manual permission review",
        ));
    }
    let descriptor = policy
        .windows_descriptor
        .as_ref()
        .ok_or(ProjectError::Invalid("missing Windows file policy"))?;
    let file = tempfile::Builder::new()
        .prefix(".rullst-update-stage-")
        .make_in(parent, |path| create(path, descriptor))?;
    // Inheritance must not silently augment or remove the approved access policy.
    // No candidate content has been written at this point.
    if describe(file.as_file())? != *descriptor {
        return Err(ProjectError::Invalid(
            "Windows inherited access policy changed; manual update required",
        ));
    }
    Ok(file)
}

fn create(path: &Path, sddl: &str) -> std::io::Result<fs::File> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let sddl: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    if path[..path.len() - 1].contains(&0) || sddl[..sddl.len() - 1].contains(&0) {
        return Err(std::io::Error::other(
            "invalid Windows path/security descriptor",
        ));
    }
    let mut descriptor = ptr::null_mut();
    // SAFETY: terminated borrowed SDDL and writable descriptor output.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    let descriptor = Allocation(descriptor);
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    // SAFETY: path and descriptor remain alive for creation. CREATE_NEW never
    // opens an existing file; policy is installed before any contents exist.
    let handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            &attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: transfer ownership of the new, uniquely owned successful handle.
    Ok(unsafe { fs::File::from_raw_handle(handle) })
}

fn plain_streams(path: &Path) -> Result<(), ProjectError> {
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    if path[..path.len() - 1].contains(&0) {
        return Err(ProjectError::Invalid("invalid Windows source path"));
    }
    let mut data = WIN32_FIND_STREAM_DATA::default();
    // SAFETY: terminated borrowed path and a correctly sized writable structure.
    let handle = unsafe {
        FindFirstStreamW(
            path.as_ptr(),
            FindStreamInfoStandard,
            (&mut data as *mut WIN32_FIND_STREAM_DATA).cast(),
            0,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let error = std::io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_HANDLE_EOF as i32) {
            Ok(())
        } else {
            Err(error.into())
        };
    }
    struct Streams(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Streams {
        fn drop(&mut self) {
            // SAFETY: owns exactly one successful stream enumeration handle.
            unsafe {
                FindClose(self.0);
            }
        }
    }
    let handle = Streams(handle);
    loop {
        let end = data
            .cStreamName
            .iter()
            .position(|unit| *unit == 0)
            .ok_or(ProjectError::Invalid("unterminated stream name"))?;
        if String::from_utf16(&data.cStreamName[..end]).ok().as_deref() != Some("::$DATA") {
            return Err(ProjectError::Invalid(
                "alternate Windows file streams require manual update",
            ));
        }
        // SAFETY: handle remains owned and data is writable for the complete call.
        if unsafe { FindNextStreamW(handle.0, (&mut data as *mut WIN32_FIND_STREAM_DATA).cast()) }
            == 0
        {
            let error = std::io::Error::last_os_error();
            return if error.raw_os_error() == Some(ERROR_HANDLE_EOF as i32) {
                Ok(())
            } else {
                Err(error.into())
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_protected_dacl_is_installed_before_candidate_bytes_and_survives_replacement() {
        for label in ["", "S:(ML;;NW;;;LW)"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("Cargo.toml");
            let sddl = format!(
                "{}{}",
                super::super::super::super::super::cache::private_file_descriptor().unwrap(),
                label
            );
            let mut original = create(&path, &sddl).unwrap();
            original.write_all(b"before").unwrap();
            drop(original);
            let policy = capture(Some(&path)).unwrap();
            assert!(!policy.windows_descriptor.as_ref().unwrap().contains('\0'));
            let mut temporary = stage(&policy, directory.path()).unwrap();
            assert_eq!(
                describe(temporary.as_file()).unwrap(),
                policy.windows_descriptor.clone().unwrap()
            );
            temporary.write_all(b"after").unwrap();
            temporary.persist(&path).unwrap();
            assert_eq!(capture(Some(&path)).unwrap(), policy);
            assert_eq!(fs::read(path).unwrap(), b"after");
        }
    }

    #[test]
    fn alternate_streams_are_rejected_without_discarding_them() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Cargo.toml");
        fs::write(&path, b"before").unwrap();
        let stream = directory.path().join("Cargo.toml:rullst-test");
        fs::write(&stream, b"retained").unwrap();
        assert!(capture(Some(&path)).is_err());
        assert_eq!(fs::read(stream).unwrap(), b"retained");
    }
}
