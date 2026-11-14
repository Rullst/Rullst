//! Windows: `GetProcessMemoryInfo` for the working set (the resident
//! memory), `GetProcessTimes` for the user and kernel time,
//! `QueryFullProcessImageNameW` for the executable and
//! `GetExtendedTcpTable` for the process listening on a port.
// Windows only; the mutation tests run on Linux.
#![allow(unsafe_code)]

use super::super::target::{Listener, single_owner};
use super::{Executable, Methods};
use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, TCP_TABLE_OWNER_PID_LISTENER,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_VM_READ, QueryFullProcessImageNameW,
};

pub(in super::super) const METHODS: Methods = Methods {
    rss: "working set after start-up, before any benchmark request (GetProcessMemoryInfo)",
    cpu: "user + kernel time of the app process during the load (GetProcessTimes)",
    executable: "file size of the running executable (QueryFullProcessImageNameW)",
    lookup: "owner of the listening socket in GetExtendedTcpTable",
    unreadable: "the process could not be opened (OpenProcess)",
};

const AF_INET: u32 = 2;
const AF_INET6: u32 = 23;
const NO_ERROR: u32 = 0;
const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
/// `MIB_TCPROW_OWNER_PID`: state, local address, local port, remote
/// address, remote port, owning PID.
const ROW_V4: Row = Row {
    size: 24,
    port: 8,
    pid: 20,
};
/// `MIB_TCP6ROW_OWNER_PID`: local address (16), scope, local port, remote
/// address (16), scope, remote port, state, owning PID.
const ROW_V6: Row = Row {
    size: 56,
    port: 20,
    pid: 52,
};

/// The byte layout of one row of a TCP table.
struct Row {
    size: usize,
    port: usize,
    pid: usize,
}

/// An open process handle, closed when dropped.
struct Process(HANDLE);

impl Process {
    fn open(pid: u32) -> Option<Self> {
        [
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
            PROCESS_QUERY_LIMITED_INFORMATION,
        ]
        .into_iter()
        .find_map(|access| {
            // SAFETY: OpenProcess takes plain values and returns a handle we
            // own (closed in Drop) or null on failure.
            let handle = unsafe { OpenProcess(access, 0, pid) };
            (!handle.is_null()).then_some(Self(handle))
        })
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: the handle came from OpenProcess and is closed only here.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub(in super::super) fn rss_bytes(pid: u32) -> Option<u64> {
    let process = Process::open(pid)?;
    let size = u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS>()).ok()?;
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: size,
        ..PROCESS_MEMORY_COUNTERS::default()
    };
    // SAFETY: the handle is open; `counters` is a live, writable
    // PROCESS_MEMORY_COUNTERS of `size` bytes.
    let ok = unsafe { K32GetProcessMemoryInfo(process.0, &raw mut counters, size) };
    (ok != 0).then(|| counters.WorkingSetSize as u64)
}

fn filetime_seconds(time: FILETIME) -> f64 {
    let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    // FILETIME counts 100-nanosecond intervals.
    ticks as f64 / 1e7
}

pub(in super::super) fn cpu_seconds(pid: u32) -> Option<f64> {
    let process = Process::open(pid)?;
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: the handle is open and the four FILETIMEs are live and
    // writable for the duration of the call.
    let ok = unsafe {
        GetProcessTimes(
            process.0,
            &raw mut creation,
            &raw mut exit,
            &raw mut kernel,
            &raw mut user,
        )
    };
    (ok != 0).then(|| filetime_seconds(kernel) + filetime_seconds(user))
}

pub(in super::super) fn executable(pid: u32) -> Option<Executable> {
    let process = Process::open(pid)?;
    let mut buffer = vec![0_u16; 32_768];
    let mut length = u32::try_from(buffer.len()).ok()?;
    // SAFETY: the handle is open; `buffer` is writable for `length` UTF-16
    // units, which the call updates to the units it wrote.
    let ok = unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &raw mut length,
        )
    };
    if ok == 0 {
        return None;
    }
    buffer.truncate(usize::try_from(length).ok()?.min(buffer.len()));
    let path = PathBuf::from(std::ffi::OsString::from_wide(&buffer));
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    Some(Executable { path, name })
}

/// The CPU model is not read on Windows.
pub(in super::super) fn cpu_model() -> Option<String> {
    None
}

/// The listening TCP table of one address family, as raw bytes.
fn tcp_table(family: u32) -> Option<Vec<u8>> {
    let mut size = 0_u32;
    for _ in 0..4 {
        // u32 words keep the table's 4-byte alignment.
        let mut words = vec![0_u32; usize::try_from(size).ok()? / 4 + 1];
        let mut capacity = u32::try_from(words.len() * 4).ok()?;
        // SAFETY: `words` is writable for `capacity` bytes; the call writes
        // at most that many, or sets `capacity` to the size it needs.
        let status = unsafe {
            GetExtendedTcpTable(
                words.as_mut_ptr().cast::<c_void>(),
                &raw mut capacity,
                0,
                family,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        };
        match status {
            NO_ERROR => return Some(words.iter().flat_map(|word| word.to_ne_bytes()).collect()),
            ERROR_INSUFFICIENT_BUFFER => size = capacity,
            _ => return None,
        }
    }
    None
}

/// The owners of the rows of `table` whose local port is `port`.
fn listening_pids(table: &[u8], row: &Row, port: u16, owners: &mut Vec<u32>) {
    let count = table
        .get(..4)
        .and_then(|bytes| bytes.try_into().ok())
        .map_or(0, u32::from_ne_bytes);
    for index in 0..usize::try_from(count).unwrap_or(0) {
        let start = 4 + index * row.size;
        let Some(bytes) = table.get(start..start + row.size) else {
            break;
        };
        // The port is in network byte order in the low 16 bits.
        let local = u16::from_be_bytes([bytes[row.port], bytes[row.port + 1]]);
        let pid = bytes
            .get(row.pid..row.pid + 4)
            .and_then(|pid| pid.try_into().ok())
            .map(u32::from_ne_bytes);
        if let Some(pid) = pid
            && local == port
            && !owners.contains(&pid)
        {
            owners.push(pid);
        }
    }
}

pub(in super::super) fn listener(port: u16) -> Listener {
    let mut owners = Vec::new();
    let mut read = false;
    for (family, row) in [(AF_INET, &ROW_V4), (AF_INET6, &ROW_V6)] {
        if let Some(table) = tcp_table(family) {
            read = true;
            listening_pids(&table, row, port, &mut owners);
        }
    }
    if !read {
        return Listener::Unknown("GetExtendedTcpTable could not list the listening sockets");
    }
    if owners.is_empty() {
        return Listener::Unknown("no listening socket found for the port");
    }
    single_owner(&owners)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_v4(port: u16, pid: u32) -> Vec<u8> {
        let mut row = vec![0_u8; ROW_V4.size];
        row[ROW_V4.port..ROW_V4.port + 2].copy_from_slice(&port.to_be_bytes());
        row[ROW_V4.pid..ROW_V4.pid + 4].copy_from_slice(&pid.to_ne_bytes());
        row
    }

    #[test]
    fn table_rows_are_matched_by_their_local_port() {
        let mut table = 3_u32.to_ne_bytes().to_vec();
        for (port, pid) in [(3000, 41), (8080, 42), (3000, 41)] {
            table.extend(row_v4(port, pid));
        }
        let mut owners = Vec::new();
        listening_pids(&table, &ROW_V4, 3000, &mut owners);
        assert_eq!(owners, [41]);
        // A truncated table stops at the last whole row.
        let mut owners = Vec::new();
        listening_pids(&table[..30], &ROW_V4, 8080, &mut owners);
        assert!(owners.is_empty());
        listening_pids(&[], &ROW_V6, 3000, &mut owners);
        assert!(owners.is_empty());
    }

    #[test]
    fn the_current_process_is_sampled() {
        let pid = std::process::id();
        let resident = rss_bytes(pid);
        assert!(resident.unwrap_or(0) > 1024 * 1024, "{resident:?}");
        let before = cpu_seconds(pid).unwrap();
        let started = std::time::Instant::now();
        let mut spin = 0u64;
        while started.elapsed() < std::time::Duration::from_millis(400) {
            spin = std::hint::black_box(spin.wrapping_add(1));
        }
        let spent = cpu_seconds(pid).unwrap() - before;
        let cpus = std::thread::available_parallelism().map_or(1, usize::from) as f64;
        let limit = started.elapsed().as_secs_f64() * cpus + 0.1;
        assert!((0.1..=limit).contains(&spent), "{spent} {limit}");
        assert_eq!((rss_bytes(u32::MAX), cpu_seconds(u32::MAX)), (None, None));
        let current = executable(pid).unwrap();
        assert_eq!(
            current.path.canonicalize().unwrap(),
            std::env::current_exe().unwrap().canonicalize().unwrap()
        );
    }

    #[test]
    fn the_process_listening_on_a_port_is_found() {
        let socket = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = socket.local_addr().unwrap().port();
        assert_eq!(listener(port), Listener::Found(std::process::id()));
    }
}
