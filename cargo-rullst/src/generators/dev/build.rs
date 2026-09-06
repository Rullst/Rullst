use std::{io, path::PathBuf, process::Stdio};
use tokio::io::{AsyncRead, AsyncReadExt};

const RECORD_LIMIT: usize = 1024 * 1024;
const DIAGNOSTIC_LIMIT: usize = 16 * 1024;

pub(super) async fn compile() -> io::Result<PathBuf> {
    compile_in(std::path::Path::new(".")).await
}

pub(super) async fn compile_in(root: &std::path::Path) -> io::Result<PathBuf> {
    let manifest: toml::Value = toml::from_str(&std::fs::read_to_string(root.join("Cargo.toml"))?)
        .map_err(io::Error::other)?;
    let package = manifest
        .get("package")
        .ok_or_else(|| io::Error::other("dev requires an application package manifest"))?;
    let package_name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .ok_or_else(|| io::Error::other("missing package.name"))?;
    let default_run = package.get("default-run").and_then(toml::Value::as_str);
    let mut command = tokio::process::Command::new("cargo");
    command.current_dir(root);
    command
        .args(["build", "--package", package_name, "--message-format=json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(binary) = default_run {
        command.args(["--bin", binary]);
    }
    super::process::configure_group(command.as_std_mut());
    let child = command.spawn()?;
    let mut build = super::process::BuildChild::new(child)?;
    let stdout = build
        .child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing Cargo output pipe"))?;
    let stderr = build
        .child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("missing Cargo error pipe"))?;
    let root_manifest = std::fs::canonicalize(root.join("Cargo.toml"))?;
    let (output, diagnostic, status) = tokio::join!(
        read_records(stdout, Some(root_manifest)),
        read_records(stderr, None),
        build.wait(),
    );
    let (executables, messages) = output?;
    let (_, errors) = diagnostic?;
    if !status?.success() {
        return Err(io::Error::other(format!(
            "Cargo build failed.\n{messages}\n{errors}"
        )));
    }
    match executables.as_slice() {
        [executable] => Ok(executable.clone()),
        [] => Err(io::Error::other(
            "Cargo produced no application executable; check the bin target",
        )),
        _ => Err(io::Error::other(
            "multiple application binaries; set package.default-run in Cargo.toml",
        )),
    }
}

async fn read_records(
    reader: impl AsyncRead + Unpin,
    root_manifest: Option<PathBuf>,
) -> io::Result<(Vec<PathBuf>, String)> {
    let mut reader = reader;
    let mut chunk = [0u8; 4096];
    let mut record = Vec::new();
    let mut oversized = false;
    let mut executables = Vec::new();
    let mut diagnostic = String::new();
    loop {
        let count = reader.read(&mut chunk).await?;
        if count == 0 {
            if !record.is_empty() && !oversized {
                consume_record(
                    &record,
                    root_manifest.as_deref(),
                    &mut executables,
                    &mut diagnostic,
                );
            }
            break;
        }
        for byte in &chunk[..count] {
            if *byte == b'\n' {
                if !oversized {
                    consume_record(
                        &record,
                        root_manifest.as_deref(),
                        &mut executables,
                        &mut diagnostic,
                    );
                }
                record.clear();
                oversized = false;
            } else if record.len() < RECORD_LIMIT {
                record.push(*byte);
            } else {
                oversized = true;
            }
        }
    }
    Ok((executables, diagnostic))
}

fn consume_record(
    record: &[u8],
    root_manifest: Option<&std::path::Path>,
    executables: &mut Vec<PathBuf>,
    diagnostic: &mut String,
) {
    if let Some(root) = root_manifest {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(record) {
            if value["reason"] == "compiler-artifact"
                && value["target"]["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
                && value["profile"]["test"] == false
                && value["manifest_path"]
                    .as_str()
                    .and_then(|path| std::fs::canonicalize(path).ok())
                    .as_deref()
                    == Some(root)
                && let Some(path) = value["executable"].as_str()
            {
                let path = PathBuf::from(path);
                if !executables.contains(&path) && executables.len() < 64 {
                    executables.push(path);
                }
            }
            if let Some(message) = value["message"]["rendered"].as_str() {
                append(diagnostic, message);
            }
        }
    } else {
        append(diagnostic, &String::from_utf8_lossy(record));
        append(diagnostic, "\n");
    }
}

fn append(output: &mut String, value: &str) {
    for character in value.chars() {
        if output.len() + character.len_utf8() > DIAGNOSTIC_LIMIT {
            break;
        }
        output.push(character);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn oversized_records_are_drained_and_diagnostics_stay_bounded() {
        let input = format!("{}\nactionable error\n", "x".repeat(RECORD_LIMIT + 10));
        let (_, diagnostic) = read_records(input.as_bytes(), None).await.unwrap();
        assert_eq!(diagnostic, "actionable error\n");
        let input = format!("{}\n", "é".repeat(100_000));
        let (_, diagnostic) = read_records(input.as_bytes(), None).await.unwrap();
        assert!(diagnostic.len() <= DIAGNOSTIC_LIMIT);
        assert!(!diagnostic.is_empty());
    }
}
