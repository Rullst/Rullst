use rullst_labs::{ContentHash, LabError as Error};
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
    SeccompRule, TargetArch,
};
use std::collections::BTreeMap;

fn filters() -> Result<[BpfProgram; 2], Error> {
    // glibc can fall back from clone3 to bounded clone/posix_spawn on ENOSYS.
    // The second filter allows clone3 only so this first denial is preserved.
    let compatibility: BpfProgram = SeccompFilter::new(
        BTreeMap::from([(libc::SYS_clone3, vec![])]),
        SeccompAction::Allow,
        SeccompAction::Errno(libc::ENOSYS as u32),
        TargetArch::x86_64,
    )
    .map_err(|_| Error::Configuration)?
    .try_into()
    .map_err(|_| Error::Configuration)?;
    let allowed = [
        libc::SYS_landlock_create_ruleset,
        libc::SYS_landlock_add_rule,
        libc::SYS_landlock_restrict_self,
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_readv,
        libc::SYS_writev,
        libc::SYS_pread64,
        libc::SYS_pwrite64,
        libc::SYS_close,
        libc::SYS_close_range,
        libc::SYS_openat,
        libc::SYS_open,
        libc::SYS_fstat,
        libc::SYS_newfstatat,
        libc::SYS_stat,
        libc::SYS_lstat,
        libc::SYS_statx,
        libc::SYS_statfs,
        libc::SYS_fstatfs,
        libc::SYS_lseek,
        libc::SYS_mmap,
        libc::SYS_mprotect,
        libc::SYS_munmap,
        libc::SYS_mremap,
        libc::SYS_madvise,
        libc::SYS_brk,
        libc::SYS_rt_sigaction,
        libc::SYS_rt_sigprocmask,
        libc::SYS_rt_sigreturn,
        libc::SYS_sigaltstack,
        libc::SYS_rt_sigsuspend,
        libc::SYS_rt_sigtimedwait,
        libc::SYS_futex,
        libc::SYS_set_robust_list,
        libc::SYS_get_robust_list,
        libc::SYS_set_tid_address,
        libc::SYS_rseq,
        libc::SYS_clock_gettime,
        libc::SYS_clock_getres,
        libc::SYS_clock_nanosleep,
        libc::SYS_nanosleep,
        libc::SYS_gettimeofday,
        libc::SYS_times,
        libc::SYS_getpid,
        libc::SYS_getppid,
        libc::SYS_gettid,
        libc::SYS_getuid,
        libc::SYS_geteuid,
        libc::SYS_getgid,
        libc::SYS_getegid,
        libc::SYS_getgroups,
        libc::SYS_sched_getaffinity,
        libc::SYS_sched_yield,
        libc::SYS_getcpu,
        libc::SYS_uname,
        libc::SYS_sysinfo,
        libc::SYS_getrandom,
        libc::SYS_access,
        libc::SYS_faccessat,
        libc::SYS_faccessat2,
        libc::SYS_readlink,
        libc::SYS_readlinkat,
        libc::SYS_getdents64,
        libc::SYS_getcwd,
        libc::SYS_chdir,
        libc::SYS_mkdir,
        libc::SYS_mkdirat,
        libc::SYS_unlink,
        libc::SYS_unlinkat,
        libc::SYS_rename,
        libc::SYS_renameat,
        libc::SYS_renameat2,
        libc::SYS_fsync,
        libc::SYS_fdatasync,
        libc::SYS_ftruncate,
        libc::SYS_fchmod,
        libc::SYS_chmod,
        libc::SYS_umask,
        libc::SYS_fallocate,
        libc::SYS_fcntl,
        libc::SYS_dup,
        libc::SYS_dup2,
        libc::SYS_dup3,
        libc::SYS_pipe,
        libc::SYS_pipe2,
        libc::SYS_poll,
        libc::SYS_ppoll,
        libc::SYS_select,
        libc::SYS_pselect6,
        libc::SYS_execve,
        libc::SYS_execveat,
        libc::SYS_clone3,
        libc::SYS_wait4,
        libc::SYS_waitid,
        libc::SYS_kill,
        libc::SYS_tgkill,
        libc::SYS_arch_prctl,
        libc::SYS_getrlimit,
        libc::SYS_getrusage,
        libc::SYS_exit,
        libc::SYS_exit_group,
    ];
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> =
        allowed.into_iter().map(|call| (call, vec![])).collect();
    // Rust's captured linker output toggles nonblocking pipe reads through
    // FIONBIO. Permit only that request, never arbitrary device-control ioctls.
    let nonblocking =
        SeccompCondition::new(1, SeccompCmpArgLen::Qword, SeccompCmpOp::Eq, libc::FIONBIO)
            .map_err(|_| Error::Configuration)?;
    rules.insert(
        libc::SYS_ioctl,
        vec![SeccompRule::new(vec![nonblocking]).map_err(|_| Error::Configuration)?],
    );
    // No additional namespaces, ptrace attachment or cgroup placement through
    // clone; process/thread creation remains subject to the hard pids controller.
    let forbidden = (libc::CLONE_NEWTIME
        | libc::CLONE_PARENT
        | libc::CLONE_NEWUSER
        | libc::CLONE_NEWNET
        | libc::CLONE_NEWNS
        | libc::CLONE_NEWPID
        | libc::CLONE_NEWIPC
        | libc::CLONE_NEWUTS
        | libc::CLONE_NEWCGROUP
        | libc::CLONE_PTRACE) as u64;
    let condition = SeccompCondition::new(
        0,
        SeccompCmpArgLen::Qword,
        SeccompCmpOp::MaskedEq(forbidden),
        0,
    )
    .map_err(|_| Error::Configuration)?;
    rules.insert(
        libc::SYS_clone,
        vec![SeccompRule::new(vec![condition]).map_err(|_| Error::Configuration)?],
    );
    let read_limit = SeccompCondition::new(2, SeccompCmpArgLen::Qword, SeccompCmpOp::Eq, 0)
        .map_err(|_| Error::Configuration)?;
    rules.insert(
        libc::SYS_prlimit64,
        vec![SeccompRule::new(vec![read_limit]).map_err(|_| Error::Configuration)?],
    );
    let set_nnp = SeccompRule::new(vec![
        SeccompCondition::new(
            0,
            SeccompCmpArgLen::Qword,
            SeccompCmpOp::Eq,
            libc::PR_SET_NO_NEW_PRIVS as u64,
        )
        .map_err(|_| Error::Configuration)?,
        SeccompCondition::new(1, SeccompCmpArgLen::Qword, SeccompCmpOp::Eq, 1)
            .map_err(|_| Error::Configuration)?,
    ])
    .map_err(|_| Error::Configuration)?;
    let get_nnp = SeccompRule::new(vec![
        SeccompCondition::new(
            0,
            SeccompCmpArgLen::Qword,
            SeccompCmpOp::Eq,
            libc::PR_GET_NO_NEW_PRIVS as u64,
        )
        .map_err(|_| Error::Configuration)?,
    ])
    .map_err(|_| Error::Configuration)?;
    rules.insert(libc::SYS_prctl, vec![set_nnp, get_nnp]);
    let strict: BpfProgram = SeccompFilter::new(
        rules,
        SeccompAction::Errno(libc::EPERM as u32),
        SeccompAction::Allow,
        TargetArch::x86_64,
    )
    .map_err(|_| Error::Configuration)?
    .try_into()
    .map_err(|_| Error::Configuration)?;
    Ok([compatibility, strict])
}
pub(super) fn apply() -> Result<(), Error> {
    // Invoked in the single-threaded worker before reading source or spawning
    // rustc; every subsequent compiler/linker thread inherits both filters.
    for filter in filters()? {
        seccompiler::apply_filter(&filter).map_err(|_| Error::Unsupported)?;
    }
    Ok(())
}
pub(super) fn fingerprint() -> Result<ContentHash, Error> {
    let mut bytes = b"RullstLabsLinuxX86Syscalls-v1".to_vec();
    for filter in filters()? {
        bytes.extend_from_slice(&(filter.len() as u64).to_le_bytes());
        for instruction in filter {
            bytes.extend_from_slice(&instruction.code.to_le_bytes());
            bytes.push(instruction.jt);
            bytes.push(instruction.jf);
            bytes.extend_from_slice(&instruction.k.to_le_bytes());
        }
    }
    Ok(ContentHash::of(&bytes))
}
