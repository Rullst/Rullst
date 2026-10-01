// src/generators/foundry/artifact.rs — The executable Cargo actually built.
//
// A fixed `target/<profile>/<name>` path misses CARGO_TARGET_DIR,
// `build.target-dir`, a workspace-level target directory and a configured
// `build.target`, and could upload a stale binary from an earlier build.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Longest JSON message read from Cargo; longer records are skipped.
const RECORD_LIMIT: usize = 1024 * 1024;

/// The package whose binary `foundry:deploy` uploads.
pub(super) struct Package<'a> {
    /// Canonical path of the package's `Cargo.toml`.
    pub(super) manifest: &'a Path,
    pub(super) name: &'a str,
    pub(super) default_run: Option<&'a str>,
}

/// Runs `cargo <arguments>` with JSON messages on stdout (diagnostics stay on
/// stderr) and returns the executable Cargo reports for `package`.
pub(super) fn build_executable(
    arguments: &[String],
    package: &Package<'_>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut child = Command::new("cargo")
        .args(arguments)
        .arg("--message-format=json-render-diagnostics")
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("missing Cargo output pipe"))?;
    let executables = package_executables(BufReader::new(stdout), package.manifest);
    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other("release build failed; deployment aborted").into());
    }
    select_executable(executables?, package)
}

/// `(target name, executable)` of every non-test binary built for `manifest`.
fn package_executables(
    mut reader: impl BufRead,
    manifest: &Path,
) -> std::io::Result<Vec<(String, PathBuf)>> {
    let mut executables = Vec::new();
    let mut record = Vec::new();
    loop {
        record.clear();
        let read = (&mut reader)
            .take(RECORD_LIMIT as u64 + 1)
            .read_until(b'\n', &mut record)?;
        if read == 0 {
            return Ok(executables);
        }
        if record.len() > RECORD_LIMIT && !record.ends_with(b"\n") {
            skip_line(&mut reader)?;
            continue;
        }
        let Ok(message) = serde_json::from_slice::<serde_json::Value>(&record) else {
            continue;
        };
        let is_package_binary = message["reason"] == "compiler-artifact"
            && message["profile"]["test"] == false
            && message["target"]["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
            && message["manifest_path"]
                .as_str()
                .and_then(|path| Path::new(path).canonicalize().ok())
                .as_deref()
                == Some(manifest);
        if let (true, Some(name), Some(executable)) = (
            is_package_binary,
            message["target"]["name"].as_str(),
            message["executable"].as_str(),
        ) {
            let executable = PathBuf::from(executable);
            if !executables.iter().any(|(_, path)| *path == executable) {
                executables.push((name.to_string(), executable));
            }
        }
    }
}

/// Discards the rest of an oversized record without buffering it.
fn skip_line(reader: &mut impl BufRead) -> std::io::Result<()> {
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return Ok(());
        }
        if let Some(position) = buffer.iter().position(|byte| *byte == b'\n') {
            reader.consume(position + 1);
            return Ok(());
        }
        let length = buffer.len();
        reader.consume(length);
    }
}

/// The only binary, else the `default-run` binary, else the one named after
/// the package.
fn select_executable(
    executables: Vec<(String, PathBuf)>,
    package: &Package<'_>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let [(_, executable)] = executables.as_slice() {
        return Ok(executable.clone());
    }
    let preferred = package.default_run.unwrap_or(package.name);
    executables
        .into_iter()
        .find(|(name, _)| name == preferred)
        .map(|(_, executable)| executable)
        .ok_or_else(|| {
            std::io::Error::other(format!(
                "Cargo reported no unambiguous executable for package `{}`; set package.default-run in Cargo.toml",
                package.name
            ))
            .into()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(manifest: &Path, name: &str, executable: &str, test: bool) -> String {
        serde_json::json!({
            "reason": "compiler-artifact",
            "manifest_path": manifest,
            "target": { "kind": ["bin"], "name": name },
            "profile": { "test": test },
            "executable": executable,
            "fresh": true,
        })
        .to_string()
    }

    #[test]
    fn the_reported_executable_is_used_wherever_cargo_built_it() {
        let directory = tempfile::tempdir().unwrap();
        let manifest = directory.path().join("Cargo.toml");
        std::fs::write(&manifest, "").unwrap();
        let other = directory.path().join("other/Cargo.toml");
        std::fs::create_dir_all(other.parent().unwrap()).unwrap();
        std::fs::write(&other, "").unwrap();
        let output = [
            "warning: not JSON".to_string(),
            artifact(&other, "dependency", "/elsewhere/dependency", false),
            artifact(
                &manifest,
                "shop",
                "/cache/x86_64-unknown-linux-musl/release/shop",
                false,
            ),
            artifact(&manifest, "shop", "/cache/test-shop", true),
            format!(
                "{{\"reason\":\"x\",\"pad\":\"{}\"}}",
                "a".repeat(RECORD_LIMIT)
            ),
        ]
        .join("\n");
        let manifest = manifest.canonicalize().unwrap();
        let executables = package_executables(output.as_bytes(), &manifest).unwrap();
        let package = Package {
            manifest: &manifest,
            name: "shop",
            default_run: None,
        };
        assert_eq!(
            select_executable(executables, &package).unwrap(),
            PathBuf::from("/cache/x86_64-unknown-linux-musl/release/shop")
        );
    }

    #[test]
    fn several_binaries_need_default_run_or_the_package_name() {
        let binaries = vec![
            ("worker".to_string(), PathBuf::from("/t/worker")),
            ("server".to_string(), PathBuf::from("/t/server")),
        ];
        let manifest = Path::new("/p/Cargo.toml");
        let named = |name, default_run| Package {
            manifest,
            name,
            default_run,
        };
        assert_eq!(
            select_executable(binaries.clone(), &named("app", Some("server"))).unwrap(),
            PathBuf::from("/t/server")
        );
        assert_eq!(
            select_executable(binaries.clone(), &named("worker", None)).unwrap(),
            PathBuf::from("/t/worker")
        );
        assert!(select_executable(binaries, &named("app", None)).is_err());
        assert!(select_executable(Vec::new(), &named("app", None)).is_err());
    }
}
