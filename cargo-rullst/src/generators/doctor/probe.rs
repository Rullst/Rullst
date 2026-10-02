//! Bounded external-program probes (`<tool> --version` and similar). Probes
//! never read standard input and run concurrently, so a slow tool does not
//! serialize the whole doctor.

use std::process::{Command, Stdio};

/// What running a program told us.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Probe {
    /// The program could be started.
    pub found: bool,
    /// It exited successfully.
    pub success: bool,
    /// Its first non-empty standard-output line, trimmed and bounded.
    pub first_line: String,
    /// Its complete standard output, bounded.
    pub stdout: String,
}

impl Probe {
    pub(crate) fn ok(&self) -> bool {
        self.found && self.success
    }
}

const OUTPUT_LIMIT: usize = 64 * 1024;

/// Runs `program args` and records the result.
pub(crate) fn run(program: &str, args: &[&str]) -> Probe {
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
    {
        Ok(output) => {
            let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            if stdout.len() > OUTPUT_LIMIT {
                let mut end = OUTPUT_LIMIT;
                while !stdout.is_char_boundary(end) {
                    end -= 1;
                }
                stdout.truncate(end);
            }
            let first_line = stdout
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or_default()
                .chars()
                .filter(|character| !character.is_control())
                .take(120)
                .collect();
            Probe {
                found: true,
                success: output.status.success(),
                first_line,
                stdout,
            }
        }
        Err(_) => Probe::default(),
    }
}

/// Runs every probe concurrently; results keep the order of `probes`.
pub(crate) fn run_all(probes: &[(&str, Vec<&str>)]) -> Vec<Probe> {
    std::thread::scope(|scope| {
        let handles: Vec<_> = probes
            .iter()
            .map(|(program, args)| scope.spawn(move || run(program, args)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap_or_default())
            .collect()
    })
}

/// The version token of `<name> <version> (...)` output, such as `1.98.1`.
pub(crate) fn version_token(line: &str) -> Option<String> {
    let token = line.split_whitespace().nth(1)?;
    token
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_digit())
        .then(|| token.trim_end_matches(',').to_string())
}

/// The installed version of `rustc` or `cargo`, for `info`.
pub(crate) fn tool_version(program: &str) -> Option<String> {
    let probe = run(program, &["--version"]);
    probe
        .ok()
        .then(|| version_token(&probe.first_line))
        .flatten()
}
