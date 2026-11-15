//! The compiled dependency count: the third-party packages Cargo builds for
//! the project, beside the `Cargo.lock` count, which also holds packages that
//! are never compiled (other platforms' dependencies, dev-dependencies, the
//! optional dependencies of features nobody enables).
//!
//! Method, on stable Cargo (1.96 and later): `cargo tree --locked --offline
//! --edges normal,build --prefix none --format {p}` in the project directory,
//! for its package. `cargo tree` uses the same feature resolver as
//! `cargo build`, for the host platform and the default features, the build
//! footprint measures; normal and build dependencies include proc macros and
//! build-script dependencies and exclude dev-dependencies. Each listed
//! `name version` that `Cargo.lock` records with a registry or git `source`
//! counts once, so path and workspace packages are left out, as in the
//! lockfile count.
use super::artifacts::find_lock;
use super::report::Metric;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const UNIT: &str = "packages";
/// `cargo tree` resolves from the local index and lock file only.
const TREE_TIMEOUT: Duration = Duration::from_secs(120);
/// Larger `cargo tree` outputs and lock files are not read.
const MAX_BYTES: u64 = 16 * 1024 * 1024;
/// The arguments after `cargo`; `--package <name>` follows when the
/// manifest names one.
pub(super) const TREE_ARGUMENTS: [&str; 11] = [
    "tree",
    "--locked",
    "--offline",
    "--edges",
    "normal,build",
    "--prefix",
    "none",
    "--format",
    "{p}",
    "--color",
    "never",
];
pub(super) const METHOD: &str = "packages with a registry or git source in Cargo.lock that `cargo tree --edges normal,build` lists for the host platform and the default features (what cargo compiles; dev-dependencies excluded)";

/// `name version` of each package with a registry or git `source` in a
/// `Cargo.lock`, or `None` when it cannot be parsed.
pub(super) fn third_party(lock: &str) -> Option<BTreeSet<(String, String)>> {
    let value: toml::Value = toml::from_str(lock).ok()?;
    let packages = value.get("package")?.as_array()?;
    Some(
        packages
            .iter()
            .filter(|package| package.get("source").is_some())
            .filter_map(|package| {
                let name = package.get("name")?.as_str()?;
                let version = package.get("version")?.as_str()?;
                Some((name.to_string(), version.to_string()))
            })
            .collect(),
    )
}

/// The distinct packages of a `cargo tree --prefix none --format {p}` output
/// that `third_party` holds. Each line is `name vVERSION`, then a path or git
/// source and the `(proc-macro)` and `(*)` markers when they apply.
pub(super) fn count_compiled(tree: &str, third_party: &BTreeSet<(String, String)>) -> u64 {
    let compiled = tree
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version = words.next()?.strip_prefix('v')?;
            Some((name.to_string(), version.to_string()))
        })
        .filter(|package| third_party.contains(package))
        .collect::<BTreeSet<_>>();
    compiled.len() as u64
}

/// Runs `command` for at most `timeout` and returns its standard output, read
/// while it runs so a large output cannot block it.
fn bounded_output(mut command: Command, timeout: Duration) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "the cargo command is not available".to_string())?;
    let stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(stdout) = stdout {
            let _ = stdout.take(MAX_BYTES).read_to_string(&mut text);
        }
        text
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let text = reader.join().unwrap_or_default();
    match status {
        Some(status) if status.success() => Ok(text),
        Some(_) => Err("`cargo tree --locked --offline` failed; build the project first (`cargo build --release`) so Cargo.lock is current and every dependency is downloaded".to_string()),
        None => Err(format!(
            "`cargo tree` did not finish within {} s",
            timeout.as_secs_f64()
        )),
    }
}

/// The compiled dependency count of the project in `start`, with `cargo` as
/// the Cargo program.
pub(super) fn count_with(cargo: &Path, start: &Path, timeout: Duration) -> Metric<u64> {
    let manifest = start.join("Cargo.toml");
    if !manifest.is_file() {
        return Metric::not_measured(
            UNIT,
            "no Cargo.toml in this directory; run footprint from the project directory",
        );
    }
    let lock = find_lock(start)
        .filter(|lock| std::fs::metadata(lock).is_ok_and(|metadata| metadata.len() <= MAX_BYTES))
        .and_then(|lock| std::fs::read_to_string(lock).ok());
    let Some(third_party) = lock.as_deref().and_then(third_party) else {
        return Metric::not_measured(UNIT, "Cargo.lock is missing or could not be parsed");
    };
    let mut command = Command::new(cargo);
    command.current_dir(start).args(TREE_ARGUMENTS);
    if let Some(package) = crate::generators::platform_name::package_name(&manifest) {
        command.args(["--package", &package]);
    }
    match bounded_output(command, timeout) {
        Ok(tree) => Metric::measured(count_compiled(&tree, &third_party), UNIT, METHOD),
        Err(reason) => Metric::not_measured(UNIT, reason),
    }
}

/// The compiled dependency count of the project around `start`.
pub(super) fn compiled_dependency_count(start: &Path) -> Metric<u64> {
    count_with(Path::new("cargo"), start, TREE_TIMEOUT)
}

#[cfg(test)]
#[path = "compiled_tests.rs"]
mod tests;
