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
        match inspect(engine, image, INSPECT_TIMEOUT) {
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

fn inspect(engine: &str, image: &str, timeout: Duration) -> Result<u64, String> {
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
    let deadline = Instant::now() + timeout;
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

        // Lock files of large workspaces are still counted.
        let large = format!("{LOCK}{}", "# padding\n".repeat(110_000));
        assert!(large.len() > 1_100_000);
        std::fs::write(root.path().join("Cargo.lock"), large).unwrap();
        assert_eq!(dependency_count(&nested).value, Some(2));

        std::fs::write(root.path().join("Cargo.lock"), "[[package]\n").unwrap();
        let unparsable = dependency_count(&nested);
        assert_eq!(
            unparsable.reason.as_deref(),
            Some("Cargo.lock could not be read or parsed")
        );
    }

    /// Runs one container-engine case when a parent test names a report
    /// file; otherwise does nothing. `PATH` holds only the fake engines.
    #[test]
    fn engine_child_fixture() {
        let Some(report) = std::env::var_os("RULLST_GREEN_ENGINE_REPORT") else {
            return;
        };
        let outcome = if std::env::var_os("RULLST_GREEN_ENGINE_HANG").is_some() {
            format!(
                "{:?}",
                inspect("docker", "app:1", Duration::from_millis(300))
            )
        } else {
            serde_json::to_string(&image_size(Some("app:1"))).unwrap()
        };
        std::fs::write(report, outcome).unwrap();
    }

    #[cfg(unix)]
    mod engines {
        use super::*;
        use crate::generators::green::tests::support::{output_within, script, sleep_program};

        /// `docker` must be asked with `--context default`, `podman` without.
        const DOCKER: &str = r#"[ "$1 $2" = "--context default" ] || exit 64
shift 2
[ "$*" = "image inspect --format {{.Size}} app:1" ] || exit 65
"$FAKE_SLEEP" 0.2
case "$FAKE_DOCKER" in
  size) echo 4242 ;;
  fail) echo 99; exit 1 ;;
  hang) exec "$FAKE_SLEEP" 30 ;;
esac"#;
        const PODMAN: &str = r#"[ "$*" = "image inspect --format {{.Size}} app:1" ] || exit 65
"$FAKE_SLEEP" 0.2
echo 777"#;

        /// The child's report for `docker` behaving as `docker` (no `docker`
        /// when `None`), a `podman` that knows the image and the variables.
        fn engines(docker: Option<&str>, podman: bool, variables: &[(&str, &str)]) -> String {
            let directory = tempfile::tempdir().unwrap();
            let bin = directory.path().join("bin");
            std::fs::create_dir(&bin).unwrap();
            if docker.is_some() {
                script(&bin, "docker", DOCKER);
            }
            if podman {
                script(&bin, "podman", PODMAN);
            }
            let report = directory.path().join("report");
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    "generators::green::artifacts::tests::engine_child_fixture",
                ])
                .env("PATH", &bin)
                .env("FAKE_SLEEP", sleep_program())
                .env("FAKE_DOCKER", docker.unwrap_or_default())
                .env("RULLST_GREEN_ENGINE_REPORT", &report)
                .env_remove("DOCKER_HOST")
                .env_remove("CONTAINER_HOST");
            for (name, value) in variables {
                command.env(name, value);
            }
            let output = output_within(command, Duration::from_secs(30));
            assert!(output.status.success(), "{output:?}");
            std::fs::read_to_string(report).unwrap()
        }

        fn metric(report: &str) -> serde_json::Value {
            serde_json::from_str(report).unwrap()
        }

        #[test]
        fn the_local_docker_engine_reports_the_image_size() {
            let measured = metric(&engines(Some("size"), false, &[]));
            assert_eq!(measured["status"], "measured");
            assert_eq!(measured["value"], 4242);
            assert_eq!(
                measured["method"],
                "docker image inspect app:1 (uncompressed size reported by the local engine)"
            );
        }

        #[test]
        fn a_remote_docker_host_is_never_asked_and_podman_is_tried_next() {
            let measured = metric(&engines(
                Some("size"),
                true,
                &[("DOCKER_HOST", "tcp://10.0.0.5:2376")],
            ));
            assert_eq!(measured["value"], 777);
            assert_eq!(
                measured["method"],
                "podman image inspect app:1 (uncompressed size reported by the local engine)"
            );

            let skipped = metric(&engines(
                Some("size"),
                true,
                &[
                    ("DOCKER_HOST", "tcp://10.0.0.5:2376"),
                    ("CONTAINER_HOST", "ssh://user@host"),
                ],
            ));
            assert_eq!(skipped["status"], "not_measured");
            assert_eq!(
                skipped["reason"],
                "DOCKER_HOST is not a local socket, so docker was not asked; CONTAINER_HOST is not a local socket, so podman was not asked"
            );
        }

        #[test]
        fn a_failed_inspection_or_a_missing_cli_is_not_measured() {
            let failed = metric(&engines(Some("fail"), false, &[]));
            assert_eq!(failed["status"], "not_measured");
            assert_eq!(failed["value"], serde_json::Value::Null);
            assert_eq!(
                failed["reason"],
                "docker has no local image `app:1` or is not running; the podman CLI is not available"
            );
        }

        #[test]
        fn a_hanging_inspection_is_stopped_at_its_deadline() {
            let report = engines(Some("hang"), false, &[("RULLST_GREEN_ENGINE_HANG", "1")]);
            assert_eq!(report, r#"Err("docker image inspect timed out")"#);
        }
    }
}
