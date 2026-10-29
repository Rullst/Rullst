//! Static figures: the executable's size, a container image's size when one
//! is named, and the dependency count from `Cargo.lock`.
use super::report::Metric;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const INSPECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Larger lock files are not parsed.
const MAX_LOCK_BYTES: u64 = 16 * 1024 * 1024;
const BYTES: &str = "bytes";

/// The file size of `executable`.
pub(super) fn binary_size(executable: Option<&Path>, method: &str) -> Metric<u64> {
    let Some(executable) = executable else {
        return Metric::not_measured(BYTES, "the app's executable is not known");
    };
    match std::fs::metadata(executable) {
        Ok(metadata) => Metric::measured(metadata.len(), BYTES, method),
        Err(_) => Metric::not_measured(BYTES, "the app's executable is not readable"),
    }
}

/// Whether a container engine host variable keeps the CLI on a local socket.
pub(super) fn local_host(value: Option<&str>) -> bool {
    match value.map(str::trim) {
        None | Some("") => true,
        Some(host) => host.starts_with("unix://") || host.starts_with("npipe://"),
    }
}

/// The engines tried, in order, with the variable that can point each one
/// at a remote host.
const ENGINES: [(&str, &str); 2] = [("docker", "DOCKER_HOST"), ("podman", "CONTAINER_HOST")];

/// `<engine> image inspect` of `image` with the first engine that knows it,
/// bounded in time; any failure is `NOT MEASURED`, never an error.
pub(super) fn image_size(image: Option<&str>) -> Metric<u64> {
    let Some(image) = image else {
        return Metric::not_measured(BYTES, "no image given; pass --image NAME to measure one");
    };
    let mut reasons = Vec::new();
    for (engine, host_variable) in ENGINES {
        if !local_host(std::env::var(host_variable).ok().as_deref()) {
            reasons.push(format!(
                "{host_variable} is not a local socket, so {engine} was not asked"
            ));
            continue;
        }
        match inspect(engine, image) {
            Ok(size) => {
                return Metric::measured(
                    size,
                    BYTES,
                    format!(
                        "{engine} image inspect {image} (uncompressed size reported by the local engine)"
                    ),
                );
            }
            Err(reason) => reasons.push(reason),
        }
    }
    Metric::not_measured(BYTES, reasons.join("; "))
}

fn inspect(engine: &str, image: &str) -> Result<u64, String> {
    let mut command = Command::new(engine);
    if engine == "docker" {
        // Never follow a configured remote context.
        command.args(["--context", "default"]);
    }
    let child = command
        .args(["image", "inspect", "--format", "{{.Size}}", image])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return Err(format!("the {engine} CLI is not available"));
    };
    let deadline = Instant::now() + INSPECT_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.by_ref().take(256).read_to_string(&mut output);
    }
    match (status, output.trim().parse::<u64>()) {
        (Some(status), Ok(size)) if status.success() => Ok(size),
        (None, _) => Err(format!("{engine} image inspect timed out")),
        _ => Err(format!(
            "{engine} has no local image `{image}` or is not running"
        )),
    }
}

/// The nearest `Cargo.lock` at or above `start`.
pub(super) fn find_lock(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join("Cargo.lock"))
        .find(|candidate| candidate.is_file())
}

/// Packages with a registry or git `source` in a `Cargo.lock`: every
/// third-party crate Cargo resolved, for all targets and features. Path and
/// workspace packages have no source and are not counted.
pub(super) fn count_dependencies(lock: &str) -> Option<u64> {
    let value: toml::Value = toml::from_str(lock).ok()?;
    let packages = value.get("package")?.as_array()?;
    let count = packages
        .iter()
        .filter(|package| package.get("source").is_some())
        .count();
    u64::try_from(count).ok()
}

/// The dependency count of the project around `start`.
pub(super) fn dependency_count(start: &Path) -> Metric<u64> {
    const UNIT: &str = "packages";
    let Some(lock) = find_lock(start) else {
        return Metric::not_measured(
            UNIT,
            "no Cargo.lock in this directory or its parents; run `cargo generate-lockfile`",
        );
    };
    let readable = std::fs::metadata(&lock).is_ok_and(|metadata| metadata.len() <= MAX_LOCK_BYTES);
    let counted = readable
        .then(|| std::fs::read_to_string(&lock).ok())
        .flatten()
        .as_deref()
        .and_then(count_dependencies);
    match counted {
        Some(count) => Metric::measured(
            count,
            UNIT,
            "packages with a registry or git source in Cargo.lock (all targets and features)",
        ),
        None => Metric::not_measured(UNIT, "Cargo.lock could not be read or parsed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = r#"
version = 4

[[package]]
name = "app"
version = "0.1.0"
dependencies = ["serde"]

[[package]]
name = "helper"
version = "0.1.0"

[[package]]
name = "serde"
version = "1.0.0"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "forked"
version = "0.2.0"
source = "git+https://example.com/forked#0123abcd"
"#;

    #[test]
    fn only_local_engine_sockets_are_used() {
        assert!(local_host(None));
        assert!(local_host(Some("")));
        assert!(local_host(Some("unix:///var/run/docker.sock")));
        assert!(local_host(Some("npipe:////./pipe/docker_engine")));
        assert!(!local_host(Some("tcp://10.0.0.5:2376")));
        assert!(!local_host(Some("ssh://user@host")));
    }

    #[test]
    fn sizes_are_measured_or_explained() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), [0_u8; 1234]).unwrap();
        let measured = binary_size(Some(file.path()), "file size");
        assert_eq!(measured.value, Some(1234));
        assert_eq!(measured.status, "measured");
        assert_eq!(measured.method.as_deref(), Some("file size"));
        let missing = binary_size(None, "file size");
        assert_eq!(missing.status, "not_measured");
        assert_eq!(
            missing.reason.as_deref(),
            Some("the app's executable is not known")
        );
        let none = image_size(None);
        assert_eq!(none.status, "not_measured");
        assert_eq!(
            none.reason.as_deref(),
            Some("no image given; pass --image NAME to measure one")
        );
    }

    #[test]
    fn third_party_packages_in_the_lock_file_are_counted() {
        assert_eq!(count_dependencies(LOCK), Some(2));
        assert_eq!(count_dependencies("version = 4\n"), None);
        assert_eq!(count_dependencies("not toml ["), None);
        assert_eq!(count_dependencies("package = []\n"), Some(0));

        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("crates/app");
        std::fs::create_dir_all(&nested).unwrap();
        // A lock file above the temporary directory would be found instead.
        if find_lock(&nested).is_none() {
            let missing = dependency_count(&nested);
            assert_eq!(missing.status, "not_measured");
            assert_eq!(missing.unit, "packages");
        }

        std::fs::write(root.path().join("Cargo.lock"), LOCK).unwrap();
        assert_eq!(find_lock(&nested), Some(root.path().join("Cargo.lock")));
        let counted = dependency_count(&nested);
        assert_eq!(counted.value, Some(2));
        assert_eq!(counted.status, "measured");

        std::fs::write(root.path().join("Cargo.lock"), "[[package]\n").unwrap();
        let unparsable = dependency_count(&nested);
        assert_eq!(
            unparsable.reason.as_deref(),
            Some("Cargo.lock could not be read or parsed")
        );
    }
}
