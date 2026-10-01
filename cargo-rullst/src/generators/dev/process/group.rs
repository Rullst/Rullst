//! A group is disarmed before its leader is reaped, never signalled by a stale ID.
#[cfg(unix)]
use std::io;
use std::process::{Command, Stdio};

pub(super) struct ProcessGroup {
    id: u32,
    armed: bool,
}

impl ProcessGroup {
    pub(super) fn new(id: u32) -> Self {
        Self { id, armed: true }
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }

    pub(super) fn terminate(&self, force: bool) {
        if !self.armed {
            return;
        }
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args([
                    if force { "-KILL" } else { "-TERM" },
                    "--",
                    &format!("-{}", self.id),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        #[cfg(windows)]
        {
            let mut command = Command::new("taskkill");
            command.args(["/PID", &self.id.to_string(), "/T"]);
            if force {
                command.arg("/F");
            }
            let _ = command.stdout(Stdio::null()).stderr(Stdio::null()).status();
        }
    }

    pub(super) fn finish(&mut self) {
        self.terminate(true);
        self.disarm();
    }

    /// Observe without waitpid: the zombie leader keeps its PID/PGID reserved
    /// until we have terminated descendants and explicitly reaped the leader.
    #[cfg(unix)]
    pub(super) fn exit_observed(&self) -> io::Result<bool> {
        #[cfg(target_os = "linux")]
        {
            let stat = std::fs::read_to_string(format!("/proc/{}/stat", self.id))?;
            let state = stat
                .rsplit_once(')')
                .and_then(|(_, fields)| fields.split_whitespace().next())
                .ok_or_else(|| io::Error::other("could not observe owned process state"))?;
            Ok(matches!(state, "Z" | "X"))
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "cygwin",
            target_os = "horizon",
            target_os = "openbsd",
            target_os = "redox"
        )))]
        {
            exited_without_reaping(self.id)
        }
        // Platforms without waitid in rustix fall back to `ps`.
        #[cfg(any(
            target_os = "cygwin",
            target_os = "horizon",
            target_os = "openbsd",
            target_os = "redox"
        ))]
        {
            let output = Command::new("ps")
                .args(["-o", "stat=", "-p", &self.id.to_string()])
                .stderr(Stdio::null())
                .output()?;
            if !output.status.success() {
                return Err(io::Error::other(
                    "could not observe owned process before reaping",
                ));
            }
            Ok(String::from_utf8_lossy(&output.stdout)
                .trim_start()
                .starts_with('Z'))
        }
    }
}

/// Whether the child `id` has exited, observed with
/// `waitid(WEXITED | WNOHANG | WNOWAIT)`: one system call that leaves the
/// zombie, and so its PID/PGID, reserved for the later reap. It replaces a
/// `ps` process spawned per 20 ms poll. A stopped child (which some kernels
/// also report) is not an exit.
#[cfg(all(
    unix,
    not(any(
        target_os = "cygwin",
        target_os = "horizon",
        target_os = "openbsd",
        target_os = "redox"
    ))
))]
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub(super) fn exited_without_reaping(id: u32) -> io::Result<bool> {
    use rustix::process::{Pid, WaitId, WaitIdOptions, waitid};
    let pid = i32::try_from(id)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or_else(|| io::Error::other("owned process ID is out of range"))?;
    let status = waitid(
        WaitId::Pid(pid),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )?;
    Ok(status.is_some_and(|status| status.exited() || status.killed() || status.dumped()))
}
