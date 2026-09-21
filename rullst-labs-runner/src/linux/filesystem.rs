use landlock::{
    ABI, Access, AccessFs, BitFlags, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset,
    RulesetAttr, RulesetCreatedAttr, RulesetStatus,
};
use rullst_labs::{ContentHash, LabError as Error};

fn rules() -> [(&'static str, BitFlags<AccessFs>); 7] {
    let read_execute = AccessFs::ReadFile | AccessFs::ReadDir | AccessFs::Execute;
    let workspace = AccessFs::ReadFile
        | AccessFs::ReadDir
        | AccessFs::WriteFile
        | AccessFs::RemoveDir
        | AccessFs::RemoveFile
        | AccessFs::MakeDir
        | AccessFs::MakeReg
        | AccessFs::MakeSym
        | AccessFs::Refer
        | AccessFs::Truncate;
    [
        ("/toolchain", read_execute),
        ("/lib", read_execute),
        ("/lib64", read_execute),
        ("/runner", AccessFs::ReadFile | AccessFs::Execute),
        ("/work", workspace),
        ("/dev/null", AccessFs::ReadFile | AccessFs::WriteFile),
        ("/dev/urandom", AccessFs::ReadFile.into()),
    ]
}
pub(super) fn fingerprint() -> Result<ContentHash, Error> {
    let description: Vec<_> = rules()
        .into_iter()
        .map(|(path, access)| (path, access.bits()))
        .collect();
    Ok(ContentHash::of(
        &serde_json::to_vec(&(
            "RullstLabsLandlockFs-v2-independent-compiler",
            3,
            AccessFs::from_all(ABI::V3).bits(),
            description,
        ))
        .map_err(|_| Error::Configuration)?,
    ))
}
/// Apply after trusted /proc/cgroup inspection but BEFORE reading source. The
/// compiler requires its separate domain to deny the interpreter's anonymous
/// descriptors; filesystem rules alone do not mediate those special inodes.
/// Only fixed tools can execute; /work is writable but not executable.
pub(super) fn enforce(report: bool) -> Result<ContentHash, Error> {
    super::probe::stage("labs-preflight:landlock-create", report);
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V3))
        .map_err(|_| Error::Unsupported)?
        .create()
        .map_err(|_| Error::Unsupported)?;
    super::probe::stage("labs-preflight:landlock-rules", report);
    for (path, access) in rules() {
        let fd = PathFd::new(path).map_err(|_| Error::Unsupported)?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, access))
            .map_err(|_| Error::Unsupported)?;
    }
    super::probe::stage("labs-preflight:landlock-restrict", report);
    let status = ruleset.restrict_self().map_err(|_| Error::Unsupported)?;
    super::probe::stage("labs-preflight:landlock-enforcement", report);
    if status.ruleset != RulesetStatus::FullyEnforced || !status.no_new_privs {
        return Err(Error::Unsupported);
    }
    for (denied, category) in [
        ("/proc/self/status", "labs-preflight:landlock-proc-denial"),
        (
            "/limits/memory.max",
            "labs-preflight:landlock-cgroup-denial",
        ),
    ] {
        super::probe::stage(category, report);
        if !matches!(std::fs::File::open(denied),Err(error) if error.raw_os_error()==Some(libc::EACCES))
        {
            return Err(Error::Unsupported);
        }
    }
    fingerprint()
}

/// Ptrace-sensitive proc entries require the target to be in our domain or a
/// descendant. A compiler in a separate Landlock domain must not reach its
/// interpreter parent, including anonymous pipes that filesystem rules ignore.
pub(super) fn deny_parent_access(parent: u32) -> Result<(), Error> {
    if rustix::process::getppid().is_none_or(|pid| pid.as_raw_pid() as u32 != parent) {
        return Err(Error::Unsupported);
    }
    for entry in ["fd/0", "fd/1", "fd/2", "mem", "ns/mnt"] {
        let path = format!("/proc/{parent}/{entry}");
        if !matches!(std::fs::File::open(path), Err(error) if matches!(error.raw_os_error(), Some(libc::EACCES | libc::EPERM)))
        {
            return Err(Error::Unsupported);
        }
    }
    let path = format!("/proc/{parent}/task/{parent}/fd/1");
    if !matches!(std::fs::File::open(path), Err(error) if matches!(error.raw_os_error(), Some(libc::EACCES | libc::EPERM)))
    {
        return Err(Error::Unsupported);
    }
    Ok(())
}
