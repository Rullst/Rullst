//! macOS: `proc_pidinfo(PROC_PIDTASKINFO)` for the resident size and the
//! user and system time (Mach absolute time units, converted with
//! `mach_timebase_info`), `proc_pidpath` for the executable and
//! `lsof -t -iTCP:<port> -sTCP:LISTEN` for the process listening on a port.
//! All of them live in libSystem, which every Rust program links.
// macOS only; the mutation tests run on Linux.
#![allow(unsafe_code)]

use super::super::target::{Listener, single_owner};
use super::{Executable, Methods};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(in super::super) const METHODS: Methods = Methods {
    rss: "resident size after start-up, before any benchmark request (proc_pidinfo PROC_PIDTASKINFO)",
    cpu: "user + system time of the app process during the load (proc_pidinfo PROC_PIDTASKINFO)",
    executable: "file size of the running executable (proc_pidpath)",
    lookup: "lsof -t -iTCP:<port> -sTCP:LISTEN",
    unreadable: "proc_pidinfo could not read the process",
};

const PROC_PIDTASKINFO: c_int = 4;
/// `PROC_PIDPATHINFO_MAXSIZE` (4 × `MAXPATHLEN`).
const PATH_MAX_SIZE: usize = 4096;
const LSOF_TIMEOUT: Duration = Duration::from_secs(5);

/// `struct proc_taskinfo` of `<sys/proc_info.h>`. The C layout needs every
/// field; only the resident size and the user and system times are read.
#[repr(C)]
#[derive(Default)]
#[allow(dead_code)]
struct ProcTaskInfo {
    virtual_size: u64,
    resident_size: u64,
    total_user: u64,
    total_system: u64,
    threads_user: u64,
    threads_system: u64,
    policy: i32,
    faults: i32,
    pageins: i32,
    cow_faults: i32,
    messages_sent: i32,
    messages_received: i32,
    syscalls_mach: i32,
    syscalls_unix: i32,
    csw: i32,
    threadnum: i32,
    numrunning: i32,
    priority: i32,
}

const _: () = assert!(std::mem::size_of::<ProcTaskInfo>() == 96);

/// `struct mach_timebase_info` of `<mach/mach_time.h>`.
#[repr(C)]
#[derive(Default)]
struct MachTimebaseInfo {
    numerator: u32,
    denominator: u32,
}

unsafe extern "C" {
    fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
    fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
    fn mach_timebase_info(info: *mut MachTimebaseInfo) -> c_int;
    fn sysctlbyname(
        name: *const c_char,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *mut c_void,
        newlen: usize,
    ) -> c_int;
}

fn task_info(pid: u32) -> Option<ProcTaskInfo> {
    let pid = c_int::try_from(pid).ok()?;
    let size = c_int::try_from(std::mem::size_of::<ProcTaskInfo>()).ok()?;
    let mut info = ProcTaskInfo::default();
    // SAFETY: `info` is a live, writable `ProcTaskInfo` of exactly `size`
    // bytes with the C layout of `struct proc_taskinfo`; proc_pidinfo writes
    // at most `size` bytes into it and returns how many it wrote.
    let written = unsafe { proc_pidinfo(pid, PROC_PIDTASKINFO, 0, (&raw mut info).cast(), size) };
    (written == size).then_some(info)
}

/// Nanoseconds per Mach absolute time unit (1 on Intel, 125/3 on Apple
/// silicon).
fn nanoseconds_per_tick() -> Option<f64> {
    let mut timebase = MachTimebaseInfo::default();
    // SAFETY: `timebase` is a live, writable `struct mach_timebase_info`;
    // the call only fills its two fields.
    let status = unsafe { mach_timebase_info(&raw mut timebase) };
    (status == 0 && timebase.denominator > 0)
        .then(|| f64::from(timebase.numerator) / f64::from(timebase.denominator))
}

pub(in super::super) fn rss_bytes(pid: u32) -> Option<u64> {
    task_info(pid).map(|info| info.resident_size)
}

pub(in super::super) fn cpu_seconds(pid: u32) -> Option<f64> {
    let info = task_info(pid)?;
    let ticks = info.total_user.checked_add(info.total_system)?;
    Some(ticks as f64 * nanoseconds_per_tick()? / 1e9)
}

pub(in super::super) fn executable(pid: u32) -> Option<Executable> {
    let pid = c_int::try_from(pid).ok()?;
    let mut buffer = vec![0_u8; PATH_MAX_SIZE];
    let capacity = u32::try_from(buffer.len()).ok()?;
    // SAFETY: `buffer` is a live, writable allocation of `capacity` bytes;
    // proc_pidpath writes at most that many and returns the path's length.
    let length = unsafe { proc_pidpath(pid, buffer.as_mut_ptr().cast(), capacity) };
    let length = usize::try_from(length).ok().filter(|length| *length > 0)?;
    buffer.truncate(length.min(PATH_MAX_SIZE));
    let path = PathBuf::from(String::from_utf8(buffer).ok()?);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    Some(Executable { path, name })
}

/// `machdep.cpu.brand_string`, such as `Apple M1`.
pub(in super::super) fn cpu_model() -> Option<String> {
    const NAME: &CStr = c"machdep.cpu.brand_string";
    let mut buffer = [0_u8; 256];
    let mut length = buffer.len();
    // SAFETY: NAME is NUL-terminated; `buffer` is writable for `length`
    // bytes, which sysctlbyname updates to the bytes it wrote; nothing is set.
    let status = unsafe {
        sysctlbyname(
            NAME.as_ptr(),
            buffer.as_mut_ptr().cast(),
            &raw mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if status != 0 {
        return None;
    }
    let text = buffer.get(..length.min(buffer.len()))?;
    let text = CStr::from_bytes_until_nul(text).ok()?.to_str().ok()?;
    let model: String = text.trim().chars().take(120).collect();
    (!model.is_empty()).then_some(model)
}

/// The PIDs `lsof -t` printed, one per line.
fn lsof_pids(output: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    for pid in output.lines().filter_map(|line| line.trim().parse().ok()) {
        if !pids.contains(&pid) {
            pids.push(pid);
        }
    }
    pids
}

pub(in super::super) fn listener(port: u16) -> Listener {
    let child = Command::new("lsof")
        .args(["-t", "-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return Listener::Unknown("lsof is not available to find the listening process");
    };
    let deadline = Instant::now() + LSOF_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Listener::Unknown("lsof did not answer within 5 seconds");
            }
        }
    }
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.by_ref().take(4096).read_to_string(&mut output);
    }
    let pids = lsof_pids(&output);
    if pids.is_empty() {
        return Listener::Unknown("lsof found no listening socket for the port");
    }
    single_owner(&pids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsof_output_is_a_list_of_distinct_pids() {
        assert_eq!(lsof_pids("4242\n"), [4242]);
        assert_eq!(lsof_pids("4242\n4243\n4242\n"), [4242, 4243]);
        assert!(lsof_pids("").is_empty());
        assert!(lsof_pids("p4242\nnot a pid\n").is_empty());
    }

    #[test]
    fn the_current_process_is_sampled() {
        let pid = std::process::id();
        let resident = rss_bytes(pid);
        assert!(resident.unwrap_or(0) > 1024 * 1024, "{resident:?}");
        // Spend measurable CPU time: Mach units that were not converted to
        // nanoseconds would be 41 times too small on Apple silicon.
        let before = cpu_seconds(pid).unwrap();
        let started = Instant::now();
        let mut spin = 0u64;
        while started.elapsed() < Duration::from_millis(400) {
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
        assert!(current.name.is_some());
        assert!(cpu_model().is_some_and(|model| !model.is_empty()));
    }

    #[test]
    fn the_process_listening_on_a_port_is_found() {
        let socket = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = socket.local_addr().unwrap().port();
        assert_eq!(listener(port), Listener::Found(std::process::id()));
    }
}
