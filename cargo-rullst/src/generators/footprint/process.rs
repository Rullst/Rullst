//! The measured process: its resident memory, cumulative CPU time and
//! executable, and the process listening on a URL's port. Linux reads
//! `/proc`, macOS asks `proc_pidinfo` (and `lsof` for the listener) and
//! Windows `GetProcessMemoryInfo`, `GetProcessTimes` and
//! `GetExtendedTcpTable`. Elsewhere every figure is `NOT MEASURED` instead
//! of guessed. Only processes of the same user are readable.

#[cfg(target_os = "linux")]
#[path = "process/linux.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "process/macos.rs"]
mod platform;
#[cfg(windows)]
#[path = "process/windows.rs"]
mod platform;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
#[path = "process/other.rs"]
mod platform;

pub(super) use platform::{METHODS, cpu_model, cpu_seconds, executable, listener, rss_bytes};

/// How this platform reads each figure, as the report states it.
#[derive(Debug)]
pub(super) struct Methods {
    /// The idle resident memory.
    pub rss: &'static str,
    /// The CPU time during the benchmark.
    pub cpu: &'static str,
    /// The size of the executable of a process found by its port.
    pub executable: &'static str,
    /// How the process listening on a port is found.
    pub lookup: &'static str,
    /// Why a known process could not be sampled.
    pub unreadable: &'static str,
}

/// The executable of a running process: the file whose size is measured and
/// the name the report shows.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Executable {
    pub path: std::path::PathBuf,
    pub name: Option<String>,
}
