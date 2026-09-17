//! The only unsafe boundary in the Windows advisory cache: owned Win32 security
//! descriptors and process-token SIDs. No catalog content enters these calls.
#[allow(unsafe_code)]
mod ffi {
    use super::super::CacheError;
    use std::{
        ffi::c_void,
        fs::File,
        os::windows::{ffi::OsStrExt, io::AsRawHandle},
        path::Path,
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, HANDLE, LocalFree},
        Security::{Authorization::*, *},
        Storage::FileSystem::*,
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    struct Allocation(*mut c_void);
    impl Drop for Allocation {
        fn drop(&mut self) {
            // SAFETY: every Allocation owns one successful LocalAlloc-family
            // Win32 result; it is never copied and outlives all interior views.
            unsafe {
                LocalFree(self.0);
            }
        }
    }
    struct Token(HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            // SAFETY: Token owns a successful OpenProcessToken result.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    pub(crate) struct Identity {
        user: Allocation,
        system: Allocation,
        admins: Allocation,
        installer: Allocation,
        user_text: String,
    }

    fn invalid(message: &'static str) -> CacheError {
        CacheError::Invalid(message)
    }
    fn os_error() -> CacheError {
        std::io::Error::last_os_error().into()
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    fn sid(text: &str) -> Result<Allocation, CacheError> {
        let value = wide(text);
        let mut result = ptr::null_mut();
        // SAFETY: input is terminated and alive; output pointer is writable.
        if unsafe { ConvertStringSidToSidW(value.as_ptr(), &mut result) } == 0 {
            return Err(os_error());
        }
        Ok(Allocation(result))
    }

    impl Identity {
        pub(crate) fn current() -> Result<Self, CacheError> {
            let mut token = ptr::null_mut();
            // SAFETY: pseudo-process handle is valid; token output is writable.
            if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
                return Err(os_error());
            }
            let token = Token(token);
            let mut bytes = 0;
            // SAFETY: null/zero requests the documented required buffer size.
            unsafe {
                GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut bytes);
            }
            if bytes == 0 || bytes > 4096 {
                return Err(invalid("invalid process-token size"));
            }
            let mut buffer = vec![0usize; (bytes as usize).div_ceil(size_of::<usize>())];
            // SAFETY: word allocation has sufficient size and alignment for
            // TOKEN_USER; Win32 writes at most the declared byte count.
            if unsafe {
                GetTokenInformation(
                    token.0,
                    TokenUser,
                    buffer.as_mut_ptr().cast(),
                    bytes,
                    &mut bytes,
                )
            } == 0
            {
                return Err(os_error());
            }
            // SAFETY: successful TokenUser result contains an aligned TOKEN_USER
            // and its SID; buffer remains alive through string conversion.
            let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
            let mut text = ptr::null_mut();
            // SAFETY: SID belongs to the validated token result; output is writable.
            if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
                return Err(os_error());
            }
            let owned = Allocation(text.cast());
            let mut length = 0;
            // SAFETY: successful ConvertSidToStringSidW allocates a terminated
            // UTF-16 SID string. Walk only that string, never caller data.
            unsafe {
                while *text.add(length) != 0 {
                    length += 1;
                }
            }
            // SAFETY: the preceding terminated-string walk establishes length;
            // the allocation is still owned here.
            let user_text = String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) })
                .map_err(|_| invalid("invalid process SID string"))?;
            drop(owned);
            Ok(Self {
                user: sid(&user_text)?,
                system: sid("S-1-5-18")?,
                admins: sid("S-1-5-32-544")?,
                installer: sid("S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464")?,
                user_text,
            })
        }

        fn trusted(&self, candidate: PSID, ancestor: bool) -> bool {
            // SAFETY: caller supplies a SID from a successful validated Win32
            // descriptor/ACE; all comparison SIDs are owned valid allocations.
            unsafe {
                EqualSid(candidate, self.user.0) != 0
                    || EqualSid(candidate, self.system.0) != 0
                    || EqualSid(candidate, self.admins.0) != 0
                    || (ancestor && EqualSid(candidate, self.installer.0) != 0)
            }
        }

        pub(crate) fn create_directory(&self, path: &Path) -> Result<(), CacheError> {
            let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if path[..path.len() - 1].contains(&0) {
                return Err(invalid("invalid cache path"));
            }
            let sddl = wide(&format!(
                "O:{}D:P(A;OICI;FA;;;{})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)",
                self.user_text, self.user_text
            ));
            let mut descriptor = ptr::null_mut();
            // SAFETY: SDDL contains only fixed syntax and an OS-produced SID;
            // terminated input and output storage live for the complete call.
            if unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    1,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            } == 0
            {
                return Err(os_error());
            }
            let descriptor = Allocation(descriptor);
            let attributes = SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.0,
                bInheritHandle: 0,
            };
            // SAFETY: CreateDirectoryW borrows both the path and descriptor only
            // during this call. ACL is installed atomically at creation.
            if unsafe { CreateDirectoryW(path.as_ptr(), &attributes) } == 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(ERROR_ALREADY_EXISTS as i32) {
                    return Err(error.into());
                }
            }
            Ok(()) // Existing entries are validated, never repaired/chmodded.
        }

        pub(crate) fn validate(
            &self,
            file: &File,
            ancestor: bool,
            private: bool,
            directory: bool,
        ) -> Result<(), CacheError> {
            let handle = file.as_raw_handle();
            let mut info = BY_HANDLE_FILE_INFORMATION::default();
            // SAFETY: file owns a valid handle; info is writable and correctly sized.
            if unsafe { GetFileInformationByHandle(handle, &mut info) } == 0 {
                return Err(os_error());
            }
            if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
                || (info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
                || (!directory && info.nNumberOfLinks != 1)
            {
                return Err(invalid(
                    "cache entry is a reparse point, hard link or unexpected type",
                ));
            }
            let mut owner = ptr::null_mut();
            let mut acl = ptr::null_mut();
            let mut descriptor = ptr::null_mut();
            // SAFETY: handle has READ_CONTROL; output pointers are writable.
            // Owner/ACL are borrowed from the returned LocalFree allocation.
            let status = unsafe {
                GetSecurityInfo(
                    handle,
                    SE_FILE_OBJECT,
                    OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                    &mut owner,
                    ptr::null_mut(),
                    &mut acl,
                    ptr::null_mut(),
                    &mut descriptor,
                )
            };
            if status != 0 {
                return Err(std::io::Error::from_raw_os_error(status as i32).into());
            }
            let descriptor = Allocation(descriptor);
            // SAFETY: successful GetSecurityInfo owns a complete descriptor.
            // Null ACL means unrestricted access, never a private cache.
            if owner.is_null()
                || acl.is_null()
                || unsafe { IsValidSid(owner) == 0 || IsValidAcl(acl) == 0 }
            {
                return Err(invalid("cache owner or DACL is missing or invalid"));
            }
            // SAFETY: owner SID was validated and all identity allocations live.
            let owned = unsafe { EqualSid(owner, self.user.0) != 0 };
            if (!ancestor && !owned) || (ancestor && !self.trusted(owner, true)) {
                return Err(invalid(
                    "cache entry is not owned by the user or a trusted OS principal",
                ));
            }
            if private && directory {
                let mut control = 0;
                let mut revision = 0;
                // SAFETY: descriptor lives through this call; outputs are writable.
                if unsafe {
                    GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision)
                } == 0
                {
                    return Err(os_error());
                }
                if control & SE_DACL_PROTECTED == 0 {
                    return Err(invalid(
                        "private cache directory must have a protected DACL",
                    ));
                }
            }
            // SAFETY: IsValidAcl succeeded on the owned descriptor.
            let count = unsafe { (*acl).AceCount };
            if count > 128 {
                return Err(invalid("cache DACL exceeds the review bound"));
            }
            for index in 0..u32::from(count) {
                let mut ace = ptr::null_mut();
                // SAFETY: index is below the validated ACL's AceCount.
                if unsafe { GetAce(acl, index, &mut ace) } == 0 {
                    return Err(os_error());
                }
                // SAFETY: GetAce returns a complete ACE_HEADER within the live ACL.
                let header = unsafe { &*ace.cast::<ACE_HEADER>() };
                if header.AceType == 1
                    || (u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0
                        && !(private && directory))
                {
                    continue;
                }
                // Only ordinary ACCESS_ALLOWED_ACE is understood. Object/callback
                // grants cannot silently become permission to use this cache.
                if header.AceType != 0 || header.AceSize < 16 {
                    return Err(invalid("unsupported cache DACL entry"));
                }
                // SAFETY: size/type establish Mask and the minimum SID header.
                let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
                let candidate = ptr::addr_of!(allowed.SidStart).cast_mut().cast::<c_void>();
                // SAFETY: byte 1 lies in the minimum 8-byte SID header checked above.
                let sub_authorities = unsafe { *candidate.cast::<u8>().add(1) };
                if 16 + usize::from(sub_authorities) * 4 > usize::from(header.AceSize) {
                    return Err(invalid("invalid cache ACE SID length"));
                }
                // SAFETY: the complete SID length is contained in this ACE.
                if unsafe { IsValidSid(candidate) } == 0 {
                    return Err(invalid("invalid cache ACE SID"));
                }
                let dangerous = DELETE
                    | WRITE_DAC
                    | WRITE_OWNER
                    | FILE_DELETE_CHILD
                    | FILE_WRITE_ATTRIBUTES
                    | 0x1000_0000
                    | 0x4000_0000;
                let forbidden = if ancestor {
                    dangerous
                } else {
                    dangerous | FILE_WRITE_DATA | FILE_APPEND_DATA | FILE_WRITE_EA
                };
                if !self.trusted(candidate, ancestor) && (private || allowed.Mask & forbidden != 0)
                {
                    return Err(invalid(
                        "cache DACL grants access to an untrusted principal",
                    ));
                }
            }
            Ok(())
        }
    }
}
pub(super) use ffi::Identity;
