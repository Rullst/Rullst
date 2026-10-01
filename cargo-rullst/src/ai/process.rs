//! Runs a validated [`Invocation`] directly (never through a shell) with
//! standard input closed, bounded captured output and a deadline.

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
    let mut child = Command::new(invocation.program()?)
        .args(&invocation.args)
        .current_dir(root)
        .env("CARGO_TERM_COLOR", "never")
        .env("NO_COLOR", "1")
        .env("RULLST_UPDATE_CHECK", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
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
    loop {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                capture.push(&line);
                on_line(String::from_utf8_lossy(&line).trim_end_matches(['\n', '\r']));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if Instant::now() > deadline {
            timed_out = true;
            let _ = child.kill();
            break;
        }
    }
    let status = child.wait()?;
    // After a timeout a grandchild may still hold the pipes; do not wait for it.
    if !timed_out {
        for reader in readers {
            let _ = reader.join();
        }
    }
    let status_text = if timed_out {
        "stopped after the time limit".to_string()
    } else {
        match status.code() {
            Some(code) => format!("exit status {code}"),
            None => "terminated by a signal".to_string(),
        }
    };
    Ok(Outcome {
        success: status.success() && !timed_out,
        status: status_text,
        output: capture.text(),
    })
}
