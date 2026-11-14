//! Linux: `/proc/<pid>/status`, `/proc/<pid>/stat` and `/proc/<pid>/exe`,
//! and the listening socket's inode matched in `/proc/<pid>/fd`.
use super::super::procfs;
use super::super::target::{Listener, single_owner};
use super::{Executable, Methods};
use std::path::PathBuf;

pub(in super::super) const METHODS: Methods = Methods {
    rss: "VmRSS after start-up, before any benchmark request (/proc/<pid>/status)",
    cpu: "utime + stime of the app process during the load (/proc/<pid>/stat)",
    executable: "file size of the running executable (/proc/<pid>/exe)",
    lookup: "/proc/net/tcp socket inode matched in /proc/<pid>/fd",
    unreadable: "/proc/<pid> was not readable",
};

/// The resident memory (`VmRSS`) in bytes.
pub(in super::super) fn rss_bytes(pid: u32) -> Option<u64> {
    procfs::sample_rss(pid)
}

/// The process's cumulative user and system CPU time in seconds.
pub(in super::super) fn cpu_seconds(pid: u32) -> Option<f64> {
    let ticks = procfs::sample_cpu_ticks(pid)?;
    procfs::cpu_seconds(0, ticks, procfs::ticks_per_second()?)
}

/// `/proc/<pid>/exe`, which stays readable when the file was replaced, and
/// the name of the file it links to.
pub(in super::super) fn executable(pid: u32) -> Option<Executable> {
    let path = PathBuf::from(format!("/proc/{pid}/exe"));
    let target = std::fs::read_link(&path).ok()?;
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    Some(Executable { path, name })
}

pub(in super::super) fn cpu_model() -> Option<String> {
    procfs::cpu_model()
}

/// Inodes of sockets listening on `port` in a `/proc/net/tcp{,6}` table.
pub(super) fn listening_inodes(table: &str, port: u16) -> Vec<u64> {
    const LISTEN: &str = "0A";
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (_, local_port) = fields.get(1)?.rsplit_once(':')?;
            let listening = *fields.get(3)? == LISTEN;
            let inode: u64 = fields.get(9)?.parse().ok()?;
            (listening && u16::from_str_radix(local_port, 16).ok()? == port && inode != 0)
                .then_some(inode)
        })
        .collect()
}

/// Finds the one process listening on `port` by matching socket inodes
/// against `/proc/<pid>/fd` links (same-user processes only).
pub(in super::super) fn listener(port: u16) -> Listener {
    let mut inodes = Vec::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(text) = std::fs::read_to_string(table) {
            inodes.extend(listening_inodes(&text, port));
        }
    }
    if inodes.is_empty() {
        return Listener::Unknown("no listening socket found for the port in /proc/net/tcp");
    }
    let targets: Vec<String> = inodes
        .iter()
        .map(|inode| format!("socket:[{inode}]"))
        .collect();
    let mut owners = Vec::new();
    let Ok(processes) = std::fs::read_dir("/proc") else {
        return Listener::Unknown("/proc is not readable");
    };
    for entry in processes.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        // Other users' descriptors are unreadable and skipped.
        let Ok(descriptors) = std::fs::read_dir(entry.path().join("fd")) else {
            continue;
        };
        let owns = descriptors.flatten().any(|descriptor| {
            std::fs::read_link(descriptor.path()).is_ok_and(|link| {
                targets
                    .iter()
                    .any(|target| link.as_os_str() == target.as_str())
            })
        });
        if owns && !owners.contains(&pid) {
            owners.push(pid);
        }
    }
    single_owner(&owners)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listening_sockets_are_found_by_port_and_state() {
        let table = include_str!("../fixtures/proc_net_tcp");
        assert_eq!(listening_inodes(table, 3000), [424242]);
        // Port 8080 only has an established connection.
        assert!(listening_inodes(table, 8080).is_empty());
        assert!(listening_inodes(table, 9).is_empty());
        let table6 = include_str!("../fixtures/proc_net_tcp6");
        assert_eq!(listening_inodes(table6, 3000), [515151]);
    }

    #[test]
    fn the_current_process_is_sampled_through_proc() {
        let pid = std::process::id();
        // Other tests allocate meanwhile, so the two readings may differ.
        let resident = rss_bytes(pid).unwrap_or(0);
        let direct = procfs::sample_rss(pid).unwrap_or(0);
        assert!(resident > 1024 * 1024, "{resident}");
        assert!(
            resident.abs_diff(direct) < 256 * 1024 * 1024,
            "{resident} {direct}"
        );
        // Spend a few clock ticks so the CPU time cannot be 0.
        let started = std::time::Instant::now();
        let mut spin = 0u64;
        while procfs::sample_cpu_ticks(pid).unwrap_or(0) < 3
            && started.elapsed() < std::time::Duration::from_secs(10)
        {
            spin = std::hint::black_box(spin.wrapping_add(1));
        }
        let rate = procfs::ticks_per_second().unwrap() as f64;
        let before = procfs::sample_cpu_ticks(pid).unwrap() as f64 / rate;
        let seconds = cpu_seconds(pid).unwrap();
        let after = procfs::sample_cpu_ticks(pid).unwrap() as f64 / rate;
        assert!(before >= 0.03, "{before}");
        assert!(
            (before..=after).contains(&seconds),
            "{before} {seconds} {after}"
        );
        assert_eq!((rss_bytes(u32::MAX), cpu_seconds(u32::MAX)), (None, None));

        let current = executable(pid).unwrap();
        assert_eq!(current.path, PathBuf::from(format!("/proc/{pid}/exe")));
        let expected = std::env::current_exe().unwrap();
        assert_eq!(
            current.name.as_deref(),
            expected.file_name().and_then(|name| name.to_str())
        );
        assert_eq!(executable(u32::MAX), None);
        assert_eq!(cpu_model(), procfs::cpu_model());
    }

    #[test]
    fn the_process_listening_on_a_port_is_found() {
        let socket = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = socket.local_addr().unwrap().port();
        assert_eq!(listener(port), Listener::Found(std::process::id()));
    }
}
