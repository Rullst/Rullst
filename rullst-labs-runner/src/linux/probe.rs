use rullst_labs::{ContentHash, LabError as Error};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};

pub(super) const MEMORY: u64 = 1_073_741_824;
pub(super) const PIDS: u64 = 32;
pub(super) const CPU: &str = "100000 100000";
pub(super) const CATEGORIES: &[&str] = &[
    "labs-preflight:seccomp",
    "labs-preflight:worker-probes",
    "labs-preflight:landlock",
    "labs-preflight:privileges",
    "labs-preflight:uid-map",
    "labs-preflight:limits",
    "labs-preflight:mounts",
    "labs-preflight:network",
    "labs-preflight:workspace",
    "labs-preflight:descriptors",
    "labs-preflight:environment",
    "labs-preflight:compiler",
    "labs-preflight:namespaces",
];
/// Stable categories from the reviewed launcher, never its raw paths/messages.
pub(super) fn launcher_failure(stderr: &str) -> Option<&'static str> {
    stderr.lines().find_map(|line| {
        let message = line.strip_prefix("bwrap: ")?;
        Some(
            if message.starts_with("No permissions to creating new namespace") {
                "labs-preflight:namespace-permission"
            } else if message.starts_with("Creating new namespace failed") {
                "labs-preflight:namespace-create"
            } else if message.starts_with("setting up uid map")
                || message.starts_with("setting up gid map")
            {
                "labs-preflight:launcher-id-map"
            } else if message.starts_with("execvp ") {
                "labs-preflight:launcher-exec"
            } else if message.starts_with("Unknown option") || message.starts_with("--") {
                "labs-preflight:launcher-options"
            } else if message.contains("mount")
                || message.contains("remount")
                || message.contains("root bind")
            {
                "labs-preflight:launcher-mount"
            } else {
                "labs-preflight:namespace-launcher"
            },
        )
    })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Observation {
    pub namespaces: BTreeMap<String, String>,
    pub syscall_policy: ContentHash,
    pub filesystem_policy: Option<ContentHash>,
    pub memory_max: u64,
    pub swap_max: u64,
    pub pids_max: u64,
    pub cpu_max: String,
    pub uid_map: String,
    pub compiler_version: String,
}
pub(super) fn namespaces() -> Result<BTreeMap<String, String>, Error> {
    ["user", "pid", "mnt", "net", "ipc", "uts", "cgroup"]
        .into_iter()
        .map(|name| {
            let value = std::fs::read_link(format!("/proc/self/ns/{name}"))
                .map_err(|_| Error::Unsupported)?;
            Ok((
                name.to_owned(),
                value.to_str().ok_or(Error::Unsupported)?.to_owned(),
            ))
        })
        .collect()
}
pub(super) fn read_text(path: impl AsRef<Path>, max: usize) -> Result<String, Error> {
    let file = std::fs::File::open(path).map_err(|_| Error::Unsupported)?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unsupported)?;
    if bytes.len() > max {
        return Err(Error::Unsupported);
    }
    String::from_utf8(bytes).map_err(|_| Error::Unsupported)
}
fn number(path: &str) -> Result<u64, Error> {
    read_text(path, 64)?
        .trim()
        .parse()
        .map_err(|_| Error::Unsupported)
}
pub(super) fn inspect() -> Result<Observation, Error> {
    eprintln!("labs-preflight:privileges");
    let status = read_text("/proc/self/status", 16384)?;
    for (name, value) in [
        ("NoNewPrivs:", "1"),
        ("CapEff:", "0000000000000000"),
        ("CapBnd:", "0000000000000000"),
        ("Seccomp:", "2"),
    ] {
        if !status
            .lines()
            .any(|line| line.strip_prefix(name).is_some_and(|v| v.trim() == value))
        {
            return Err(Error::Unsupported);
        }
    }
    // Bubblewrap's --disable-userns deliberately creates a second user
    // namespace; its one-ID map can therefore map 0 to its parent namespace's
    // 0. The controller independently checks a different user namespace inode.
    eprintln!("labs-preflight:uid-map");
    let uid_map = read_text("/proc/self/uid_map", 1024)?;
    let mapping: Vec<u64> = uid_map
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|_| Error::Unsupported)?;
    if !matches!(mapping.as_slice(), [0, _, 1]) {
        return Err(Error::Unsupported);
    }
    eprintln!("labs-preflight:limits");
    let fs = rustix::fs::statfs("/limits").map_err(|_| Error::Unsupported)?;
    if fs.f_type != libc::CGROUP2_SUPER_MAGIC {
        return Err(Error::Unsupported);
    }
    if std::fs::OpenOptions::new()
        .write(true)
        .open("/limits/memory.max")
        .is_ok()
    {
        return Err(Error::Unsupported);
    }
    let own = std::process::id().to_string();
    if !read_text("/limits/cgroup.procs", 4096)?
        .lines()
        .any(|pid| pid == own)
    {
        return Err(Error::Unsupported);
    }
    let memory_max = number("/limits/memory.max")?;
    let swap_max = number("/limits/memory.swap.max")?;
    let pids_max = number("/limits/pids.max")?;
    let cpu_max = read_text("/limits/cpu.max", 64)?.trim().to_owned();
    if memory_max != MEMORY || swap_max != 0 || pids_max != PIDS || cpu_max != CPU {
        return Err(Error::Unsupported);
    }
    eprintln!("labs-preflight:mounts");
    for path in [
        "/home",
        "/root",
        "/run",
        "/var/run/docker.sock",
        "/sys",
        "/etc/shadow",
    ] {
        if std::fs::symlink_metadata(path).is_ok() {
            return Err(Error::Unsupported);
        }
    }
    eprintln!("labs-preflight:network");
    let denied = std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([169, 254, 169, 254], 80)),
        std::time::Duration::from_millis(100),
    );
    if !matches!(denied,Err(error) if error.raw_os_error()==Some(libc::EPERM)) {
        return Err(Error::Unsupported);
    }
    if std::fs::OpenOptions::new()
        .write(true)
        .open("/toolchain/bin/rustc")
        .is_ok()
    {
        return Err(Error::Unsupported);
    }
    eprintln!("labs-preflight:workspace");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open("/work/probe")
        .map_err(|_| Error::Unsupported)?;
    file.write_all(b"bounded-workspace-probe")
        .map_err(|_| Error::Unsupported)?;
    drop(file);
    std::fs::remove_file("/work/probe").map_err(|_| Error::Unsupported)?;
    eprintln!("labs-preflight:descriptors");
    // The directory iterator may own a descriptor for /proc/self/fd itself.
    for entry in std::fs::read_dir("/proc/self/fd").map_err(|_| Error::Unsupported)? {
        let entry = entry.map_err(|_| Error::Unsupported)?;
        let fd = entry
            .file_name()
            .to_str()
            .ok_or(Error::Unsupported)?
            .parse::<u32>()
            .map_err(|_| Error::Unsupported)?;
        if fd > 2
            && !std::fs::read_link(entry.path())
                .map_err(|_| Error::Unsupported)?
                .to_string_lossy()
                .ends_with("/fd")
        {
            return Err(Error::Unsupported);
        }
    }
    eprintln!("labs-preflight:environment");
    if std::env::vars_os().any(|(key, _)| {
        !matches!(
            key.to_str(),
            Some("PATH" | "LANG" | "LC_ALL" | "TMPDIR" | "LD_LIBRARY_PATH" | "RAYON_NUM_THREADS")
        )
    }) {
        return Err(Error::Unsupported);
    }
    eprintln!("labs-preflight:compiler");
    let mut compiler = std::process::Command::new("/toolchain/bin/rustc")
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| Error::Unsupported)?;
    let mut version = String::new();
    compiler
        .stdout
        .take()
        .ok_or(Error::Unsupported)?
        .take(257)
        .read_to_string(&mut version)
        .map_err(|_| Error::Unsupported)?;
    if version.len() > 256
        || !version.starts_with("rustc 1.96.0 ")
        || !compiler.wait().map_err(|_| Error::Unsupported)?.success()
    {
        return Err(Error::Unsupported);
    }
    eprintln!("labs-preflight:namespaces");
    Ok(Observation {
        namespaces: namespaces()?,
        syscall_policy: super::syscalls::fingerprint()?,
        filesystem_policy: None,
        memory_max,
        swap_max,
        pids_max,
        cpu_max,
        uid_map,
        compiler_version: version.trim().to_owned(),
    })
}
