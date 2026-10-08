//! Static sizes: the measured executable and, when a local Docker daemon
//! knows it, the image named after the project.
use super::report::Metric;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const DOCKER_TIMEOUT: Duration = Duration::from_secs(10);

/// The file size of `executable`.
pub(super) fn binary_size(executable: Option<&Path>, method: &str) -> Metric<u64> {
    let Some(executable) = executable else {
        return Metric::not_measured("bytes", "the app's executable is not known");
    };
    match std::fs::metadata(executable) {
        Ok(metadata) => Metric::measured(metadata.len(), "bytes", method),
        Err(_) => Metric::not_measured("bytes", "the app's executable is not readable"),
    }
}

/// Whether `DOCKER_HOST` keeps the Docker CLI on a local socket.
pub(super) fn local_docker_host(value: Option<&str>) -> bool {
    match value.map(str::trim) {
        None | Some("") => true,
        Some(host) => host.starts_with("unix://") || host.starts_with("npipe://"),
    }
}

/// `docker image inspect` of the image named after the project, bounded in
/// time; any failure is `NOT MEASURED`, never an error.
pub(super) fn docker_image_size(image: Option<&str>) -> Metric<u64> {
    const UNIT: &str = "bytes";
    let Some(image) = image else {
        return Metric::not_measured(UNIT, "no project name to derive an image name from");
    };
    if !local_docker_host(std::env::var("DOCKER_HOST").ok().as_deref()) {
        return Metric::not_measured(
            UNIT,
            "DOCKER_HOST is not a local socket; footprint makes no remote calls",
        );
    }
    let child = Command::new("docker")
        .args([
            "--context",
            "default",
            "image",
            "inspect",
            "--format",
            "{{.Size}}",
            image,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        return Metric::not_measured(UNIT, "the docker CLI is not available");
    };
    let deadline = Instant::now() + DOCKER_TIMEOUT;
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
        (Some(status), Ok(size)) if status.success() => Metric::measured(
            size,
            UNIT,
            format!(
                "docker image inspect {image} (uncompressed size reported by the local daemon)"
            ),
        ),
        (None, _) => Metric::not_measured(UNIT, "docker image inspect timed out"),
        _ => Metric::not_measured(
            UNIT,
            format!(
                "docker image inspect found no local image `{image}` or the daemon is unreachable"
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_local_docker_sockets_are_used() {
        assert!(local_docker_host(None));
        assert!(local_docker_host(Some("")));
        assert!(local_docker_host(Some("unix:///var/run/docker.sock")));
        assert!(local_docker_host(Some("npipe:////./pipe/docker_engine")));
        assert!(!local_docker_host(Some("tcp://10.0.0.5:2376")));
        assert!(!local_docker_host(Some("ssh://user@host")));
    }

    #[test]
    fn sizes_are_measured_or_explained() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), [0_u8; 1234]).unwrap();
        let measured = binary_size(Some(file.path()), "file size");
        assert_eq!(measured.value, Some(1234));
        assert_eq!(measured.status, "measured");
        let missing = binary_size(None, "file size");
        assert_eq!(missing.status, "not_measured");
        assert!(missing.reason.is_some());
        assert_eq!(docker_image_size(None).status, "not_measured");
    }
}
