//! The only processes the assistant may start: an allowlisted `cargo rullst`
//! subcommand, `cargo check` or `cargo test`. Programs are executed directly
//! (never through a shell) with arguments that pass a strict token grammar,
//! standard input closed, bounded captured output and a deadline.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// `cargo rullst` subcommands the assistant may propose. Deliberately absent:
/// deploy, foundry:*, upgrade, update, pkg, new, dev, dash, studio, build*,
/// omni, eject, hook:install, db:migrate/rollback/seed, auth and
/// generate:models (which would route a database connection string through
/// the model).
pub(super) const RULLST_ALLOWLIST: &[&str] = &[
    "make:controller",
    "make:model",
    "make:resource",
    "make:middleware",
    "make:migration",
    "make:migration:auto",
    "make:billing",
    "make:omni",
    "make:iot",
    "make:mail",
    "make:mail-invoice",
    "make:mail-dunning",
    "make:cors",
    "make:jwt",
    "make:worker",
    "make:island",
    "make:chat-session",
    "make:k8s",
    "make:mfa",
    "make:scalar",
    "make:live",
    "make:grpc",
    "make:privacy",
    "make:age-gate",
    "generate:openapi",
    "generate:ts",
    "generate:diagram",
    "generate:ai-context",
    "generate:api",
    "generate:buildah",
    "db:status",
    "doctor",
    "audit",
    "inspect",
];

/// Flags refused even on allowlisted commands: `doctor --fix` installs
/// software and `audit --network` scans local listeners.
const FORBIDDEN_FLAGS: &[(&str, &str)] = &[("doctor", "--fix"), ("audit", "--network")];

const CARGO_CHECK_FLAGS: &[&str] = &[
    "--all-targets",
    "--tests",
    "--lib",
    "--bins",
    "--all-features",
    "--quiet",
    "-q",
    "--message-format=short",
];
const CARGO_TEST_FLAGS: &[&str] = &[
    "--lib",
    "--tests",
    "--all-targets",
    "--all-features",
    "--no-fail-fast",
    "--quiet",
    "-q",
];
const MAX_ARGS: usize = 8;
const MAX_ARG_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum CommandKind {
    /// `cargo rullst <args>`
    Rullst,
    /// `cargo check|test <args>`
    Cargo,
}

/// A validated invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Invocation {
    pub kind: CommandKind,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum CommandError {
    #[error("`{0}` is not an allowlisted command")]
    NotAllowed(String),
    #[error("argument `{0}` is not allowed")]
    Argument(String),
    #[error("a command needs a subcommand and at most 8 arguments")]
    Arity,
}

/// One argument token: no whitespace, quotes, shell metacharacters, `..`,
/// absolute paths or home-relative paths. Programs are not run through a
/// shell; the grammar keeps the displayed command unambiguous.
fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ARG_BYTES
        && !value.contains("..")
        && !value.starts_with('/')
        && !value.starts_with('~')
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'_' | b'-' | b'.' | b'/' | b':' | b'=' | b',' | b'@' | b'+'
                )
        })
}

/// A flag (`-m`, `--api`, `--name=value`) or a positional value.
fn safe_argument(value: &str) -> bool {
    if !safe_token(value) {
        return false;
    }
    match value.strip_prefix("--").or_else(|| value.strip_prefix('-')) {
        Some(flag) => {
            let name = flag.split_once('=').map_or(flag, |(name, _)| name);
            name.starts_with(|c: char| c.is_ascii_alphabetic())
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        }
        None => true,
    }
}

fn arity(args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() || args.len() > MAX_ARGS + 1 {
        Err(CommandError::Arity)
    } else {
        Ok(())
    }
}

/// Validates `cargo rullst <args>`.
pub(super) fn validate_rullst(args: Vec<String>) -> Result<Invocation, CommandError> {
    arity(&args)?;
    let subcommand = args[0].as_str();
    if !RULLST_ALLOWLIST.contains(&subcommand) {
        return Err(CommandError::NotAllowed(format!(
            "cargo rullst {}",
            truncate(subcommand)
        )));
    }
    for argument in &args[1..] {
        let flag = argument
            .split_once('=')
            .map_or(argument.as_str(), |(flag, _)| flag);
        if !safe_argument(argument)
            || FORBIDDEN_FLAGS
                .iter()
                .any(|(command, forbidden)| *command == subcommand && *forbidden == flag)
        {
            return Err(CommandError::Argument(truncate(argument)));
        }
    }
    Ok(Invocation {
        kind: CommandKind::Rullst,
        args,
    })
}

/// Validates `cargo check|test <args>`. `test` accepts one test-name filter
/// and `--test <name>`; nothing may redirect the manifest, target directory,
/// configuration or toolchain.
pub(super) fn validate_cargo(args: Vec<String>) -> Result<Invocation, CommandError> {
    arity(&args)?;
    let (allowed, test) = match args[0].as_str() {
        "check" => (CARGO_CHECK_FLAGS, false),
        "test" => (CARGO_TEST_FLAGS, true),
        other => {
            return Err(CommandError::NotAllowed(format!(
                "cargo {}",
                truncate(other)
            )));
        }
    };
    let identifier = |value: &str| {
        !value.is_empty()
            && value.len() <= MAX_ARG_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
    };
    let mut filter_seen = false;
    let mut iter = args[1..].iter();
    while let Some(argument) = iter.next() {
        if allowed.contains(&argument.as_str()) {
            continue;
        }
        if test && argument == "--test" {
            match iter.next() {
                Some(name) if identifier(name) => continue,
                _ => return Err(CommandError::Argument("--test".to_string())),
            }
        }
        if test && !filter_seen && identifier(argument) {
            filter_seen = true;
            continue;
        }
        return Err(CommandError::Argument(truncate(argument)));
    }
    Ok(Invocation {
        kind: CommandKind::Cargo,
        args,
    })
}

fn truncate(value: &str) -> String {
    let mut output: String = value.chars().take(64).collect();
    if output.len() < value.len() {
        output.push('…');
    }
    super::term::sanitize(&output)
}

impl Invocation {
    /// The exact command line shown to the user.
    pub(super) fn display(&self) -> String {
        let program = match self.kind {
            CommandKind::Rullst => "cargo rullst",
            CommandKind::Cargo => "cargo",
        };
        format!("{program} {}", self.args.join(" "))
    }

    /// Whether the command may write project files (and so needs a checkpoint).
    pub(super) fn mutates(&self) -> bool {
        match self.kind {
            CommandKind::Cargo => false,
            CommandKind::Rullst => match self.args.first().map(String::as_str) {
                Some("db:status" | "doctor" | "inspect") => false,
                Some("audit") => self
                    .args
                    .iter()
                    .any(|arg| arg == "--compliance" || arg == "--sbom"),
                _ => true,
            },
        }
    }

    fn deadline(&self) -> Duration {
        match self.kind {
            CommandKind::Rullst => Duration::from_secs(5 * 60),
            CommandKind::Cargo => Duration::from_secs(30 * 60),
        }
    }

    fn program(&self) -> std::io::Result<std::path::PathBuf> {
        match self.kind {
            CommandKind::Rullst => std::env::current_exe(),
            // Cargo sets `CARGO` for subcommands; otherwise use the PATH entry.
            CommandKind::Cargo => Ok(std::env::var_os("CARGO")
                .map(std::path::PathBuf::from)
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| "cargo".into())),
        }
    }
}

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

#[cfg(test)]
#[path = "tests/commands.rs"]
mod tests;
