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
        #[cfg(not(target_os = "linux"))]
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
