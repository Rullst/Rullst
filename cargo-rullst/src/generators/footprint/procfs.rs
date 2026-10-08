//! Process CPU time and memory from Linux `/proc`. Other platforms report
//! `NOT MEASURED` instead of guessing.

/// `utime + stime` clock ticks from the text of `/proc/<pid>/stat`.
///
/// The command name (field 2) may contain spaces and `)`, so fields are
/// counted after the last `)`: state is field 3, utime 14 and stime 15.
pub(super) fn cpu_ticks(stat: &str) -> Option<u64> {
    let (_, rest) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let user: u64 = fields.get(11)?.parse().ok()?;
    let system: u64 = fields.get(12)?.parse().ok()?;
    user.checked_add(system)
}

/// A `kB` field such as `VmHWM` from `/proc/<pid>/status`, in bytes.
pub(super) fn status_bytes(status: &str, key: &str) -> Option<u64> {
    status.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name != key {
            return None;
        }
        let kib: u64 = value.trim().strip_suffix("kB")?.trim().parse().ok()?;
        kib.checked_mul(1024)
    })
}

/// Clock ticks per second used by `/proc/<pid>/stat` (`_SC_CLK_TCK`).
#[cfg(unix)]
pub(super) fn ticks_per_second() -> Option<u64> {
    Some(rustix::param::clock_ticks_per_second()).filter(|ticks| *ticks > 0)
}

#[cfg(not(unix))]
pub(super) fn ticks_per_second() -> Option<u64> {
    None
}

fn read(pid: u32, file: &str) -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    std::fs::read_to_string(format!("/proc/{pid}/{file}")).ok()
}

/// The process's cumulative CPU ticks, when readable.
pub(super) fn sample_cpu_ticks(pid: u32) -> Option<u64> {
    read(pid, "stat").as_deref().and_then(cpu_ticks)
}

/// `(VmHWM, VmRSS)` in bytes, when readable.
pub(super) fn sample_memory(pid: u32) -> (Option<u64>, Option<u64>) {
    match read(pid, "status") {
        Some(status) => (
            status_bytes(&status, "VmHWM"),
            status_bytes(&status, "VmRSS"),
        ),
        None => (None, None),
    }
}

/// CPU seconds between two tick samples.
pub(super) fn cpu_seconds(start: u64, end: u64, ticks_per_second: u64) -> Option<f64> {
    let ticks = end.checked_sub(start)?;
    (ticks_per_second > 0).then(|| ticks as f64 / ticks_per_second as f64)
}

/// The first `model name` of `/proc/cpuinfo` (Linux), for the report header.
pub(super) fn cpu_model() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    cpuinfo_model(&cpuinfo)
}

fn cpuinfo_model(cpuinfo: &str) -> Option<String> {
    cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "model name").then(|| value.trim().chars().take(120).collect())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_cpu_ticks_survive_parentheses_in_the_command_name() {
        assert_eq!(
            cpu_ticks(include_str!("fixtures/proc_pid_stat")),
            Some(1234 + 567)
        );
        assert_eq!(cpu_ticks("1 (init) S 0 1"), None);
        assert_eq!(cpu_ticks("no parenthesis"), None);
        assert_eq!(cpu_ticks("1 (x) S 0 0 0 0 0 0 0 0 0 0 abc 5"), None);
    }

    #[test]
    fn status_fields_are_parsed_in_bytes() {
        let status = include_str!("fixtures/proc_pid_status");
        assert_eq!(status_bytes(status, "VmHWM"), Some(18_432 * 1024));
        assert_eq!(status_bytes(status, "VmRSS"), Some(16_384 * 1024));
        assert_eq!(status_bytes(status, "VmSwap"), None);
        // Threads has no unit and is not a memory field.
        assert_eq!(status_bytes(status, "Threads"), None);
        assert_eq!(status_bytes("VmHWM:\t  x kB", "VmHWM"), None);
    }

    #[test]
    fn cpu_seconds_use_the_clock_tick_rate() {
        assert_eq!(cpu_seconds(100, 350, 100), Some(2.5));
        assert_eq!(cpu_seconds(350, 100, 100), None);
        assert_eq!(cpu_seconds(0, 10, 0), None);
    }

    #[test]
    fn the_cpu_model_is_the_first_model_name() {
        let cpuinfo = "processor\t: 0\nmodel name\t: Example CPU @ 2.0GHz\nprocessor\t: 1\nmodel name\t: Other\n";
        assert_eq!(
            cpuinfo_model(cpuinfo).as_deref(),
            Some("Example CPU @ 2.0GHz")
        );
        assert_eq!(cpuinfo_model("processor\t: 0\n"), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_current_process_can_be_sampled() {
        let pid = std::process::id();
        assert!(sample_cpu_ticks(pid).is_some());
        let (peak, resident) = sample_memory(pid);
        assert!(peak.unwrap_or(0) >= resident.unwrap_or(0));
        assert!(resident.unwrap_or(0) > 0);
        assert!(ticks_per_second().is_some());
    }
}
