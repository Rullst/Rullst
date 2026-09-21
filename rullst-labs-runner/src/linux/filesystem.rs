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
            "RullstLabsLandlockFs-v1",
            3,
            AccessFs::from_all(ABI::V3).bits(),
            description,
        ))
        .map_err(|_| Error::Configuration)?,
    ))
}
/// Apply after trusted /proc/cgroup inspection but BEFORE reading source. The
/// compiler can no longer reopen the worker's descriptors/memory via /proc.
/// Only fixed tools can execute; /work is writable but not executable.
pub(super) fn enforce() -> Result<ContentHash, Error> {
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::from_all(ABI::V3))
        .map_err(|_| Error::Unsupported)?
        .create()
        .map_err(|_| Error::Unsupported)?;
    for (path, access) in rules() {
        let fd = PathFd::new(path).map_err(|_| Error::Unsupported)?;
        ruleset = ruleset
            .add_rule(PathBeneath::new(fd, access))
            .map_err(|_| Error::Unsupported)?;
    }
    let status = ruleset.restrict_self().map_err(|_| Error::Unsupported)?;
    if status.ruleset != RulesetStatus::FullyEnforced || !status.no_new_privs {
        return Err(Error::Unsupported);
    }
    for denied in ["/proc/self/status", "/proc/self/fd/1", "/limits/memory.max"] {
        if !matches!(std::fs::File::open(denied),Err(error) if error.raw_os_error()==Some(libc::EACCES))
        {
            return Err(Error::Unsupported);
        }
    }
    fingerprint()
}
