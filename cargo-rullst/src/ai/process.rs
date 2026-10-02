//! Runs a validated [`Invocation`] directly (never through a shell) with
//! standard input closed, bounded captured output and a deadline.
//!
//! The run ends when the command exits, not when its output pipes close: a
//! process it started that keeps them open (a test server, say) delays the
//! result by at most [`EXIT_GRACE`]. On Unix the command runs in its own
//! process group, so a timeout, or anything left holding its output after
//! it exits, stops everything it started; Ctrl+C is forwarded to that group
//! because the terminal no longer delivers it there. On Windows only the
//! command itself is stopped.

use super::commands::Invocation;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Head and tail of a stream, bounded regardless of its length.
#[derive(Default)]
pub(super) struct Capture {
    head: Vec<u8>,
    tail: std::collections::VecDeque<u8>,
    omitted: usize,
}

const HEAD_BYTES: usize = 6 * 1024;
const TAIL_BYTES: usize = 10 * 1024;
/// How long output may still arrive after the command exited.
pub(super) const EXIT_GRACE: Duration = Duration::from_secs(2);

impl Capture {
    pub(super) fn push(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if self.head.len() < HEAD_BYTES {
                self.head.push(byte);
            } else {
                self.tail.push_back(byte);
                if self.tail.len() > TAIL_BYTES {
                    self.tail.pop_front();
                    self.omitted += 1;
                }
            }
        }
    }

    pub(super) fn text(&self) -> String {
        let mut text = String::from_utf8_lossy(&self.head).into_owned();
        if self.omitted > 0 {
            text.push_str(&format!("\n… {} bytes omitted …\n", self.omitted));
        }
        let tail: Vec<u8> = self.tail.iter().copied().collect();
        text.push_str(&String::from_utf8_lossy(&tail));
        text
    }
}

/// The outcome of one executed command.
pub(super) struct Outcome {
    pub success: bool,
    pub status: String,
    pub output: String,
}

fn spawn_reader<R: Read + Send + 'static>(
    stream: R,
    sender: mpsc::Sender<Vec<u8>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stream);
        let mut line = Vec::new();
        loop {
            line.clear();
            match std::io::BufRead::read_until(&mut reader, b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if sender.send(line.clone()).is_err() {
                        break;
                    }
                }
            }
        }
    })
}

/// Runs `invocation` in `root`, forwarding output lines to `on_line` as they
/// arrive. The child's stdin is closed so it can never wait for input.
pub(super) fn run(
    invocation: &Invocation,
    root: &Path,
    mut on_line: impl FnMut(&str),
) -> std::io::Result<Outcome> {
    let mut command = Command::new(invocation.program()?);
    command
        .args(&invocation.args)
        .current_dir(root)
        .env("CARGO_TERM_COLOR", "never")
        .env("NO_COLOR", "1")
        .env("RULLST_UPDATE_CHECK", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command.spawn()?;
    #[cfg(unix)]
    let _interrupts = group::forward_interrupts(&child);
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        readers.push(spawn_reader(stdout, sender.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        readers.push(spawn_reader(stderr, sender.clone()));
    }
    drop(sender);
    let deadline = Instant::now() + invocation.deadline();
    let mut capture = Capture::default();
    let mut timed_out = false;
    let mut lingering = false;
    let mut exited_at: Option<Instant> = None;
    loop {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                capture.push(&line);
                on_line(String::from_utf8_lossy(&line).trim_end_matches(['\n', '\r']));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if exited_at.is_none() && child.try_wait()?.is_some() {
            exited_at = Some(Instant::now());
        }
        if exited_at.is_some_and(|exited| exited.elapsed() > EXIT_GRACE) {
            // Something the command started still holds its output.
            lingering = true;
            stop(&mut child);
            break;
        }
        if Instant::now() > deadline {
            timed_out = true;
            stop(&mut child);
            break;
        }
    }
    let status = child.wait()?;
    // A process that ignored the stop may still hold the pipes; the reader
    // threads are then left to end with it instead of being joined.
    if !timed_out && !lingering {
        for reader in readers {
            let _ = reader.join();
        }
    }
    let mut status_text = if timed_out {
        "stopped after the time limit".to_string()
    } else {
        match status.code() {
            Some(code) => format!("exit status {code}"),
            None => "terminated by a signal".to_string(),
        }
    };
    if lingering {
        status_text.push_str(if cfg!(unix) {
            " (processes it left running were stopped)"
        } else {
            " (processes it left running still hold its output)"
        });
    }
    Ok(Outcome {
        success: status.success() && !timed_out,
        status: status_text,
        output: capture.text(),
    })
}

/// Stops the command and, on Unix, every process in its group.
fn stop(child: &mut std::process::Child) {
    #[cfg(unix)]
    group::signal(child, rustix::process::Signal::KILL);
    let _ = child.kill();
}

#[cfg(unix)]
mod group {
    use std::process::Child;
    use std::thread::JoinHandle;
    use tokio::sync::oneshot;

    /// Sends `signal` to the process group the command leads.
    pub(super) fn signal(child: &Child, signal: rustix::process::Signal) {
        let pid = rustix::process::Pid::from_child(child);
        // `kill(-1)` would signal every process the user owns.
        if pid.as_raw_nonzero().get() > 1 {
            let _ = rustix::process::kill_process_group(pid, signal);
        }
    }

    /// Forwards Ctrl+C to the command's group while it is alive. The command
    /// is not in the terminal's foreground group, so it would not see it.
    pub(super) struct Interrupts {
        stop: Option<oneshot::Sender<()>>,
        thread: Option<JoinHandle<()>>,
    }

    pub(super) fn forward_interrupts(child: &Child) -> Interrupts {
        let pid = rustix::process::Pid::from_child(child);
        let (stop, stopped) = oneshot::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("rullst-ai-interrupts".to_string())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                runtime.block_on(async move {
                    tokio::select! {
                        result = tokio::signal::ctrl_c() => {
                            if result.is_ok() && pid.as_raw_nonzero().get() > 1 {
                                let _ = rustix::process::kill_process_group(
                                    pid,
                                    rustix::process::Signal::INT,
                                );
                            }
                        }
                        _ = stopped => {}
                    }
                });
            })
            .ok();
        Interrupts {
            stop: Some(stop),
            thread,
        }
    }

    impl Drop for Interrupts {
        fn drop(&mut self) {
            drop(self.stop.take());
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
