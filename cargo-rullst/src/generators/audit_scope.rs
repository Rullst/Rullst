//! Which source trees the project-source audit scans cover.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::generators::audit_compliance::EvidenceStatus;
use crate::generators::source_walk::rust_sources;

/// `src` of the package at `root` plus the `src` of every workspace member
/// below `root`, or `None` when `root` has no `src` directory (such as a
/// virtual workspace root, which callers walk whole).
///
/// Members come from `cargo metadata --no-deps`; when Cargo cannot describe
/// the workspace only the package's own `src` is returned.
pub(super) fn package_source_roots(root: &Path) -> Option<Vec<PathBuf>> {
    let src = under(root, Path::new("src"));
    if !src.is_dir() {
        return None;
    }
    let mut roots = vec![src];
    let canonical_root = root.canonicalize().ok();
    for member in canonical_root
        .as_deref()
        .map(workspace_member_directories)
        .unwrap_or_default()
    {
        let Some(relative) = canonical_root
            .as_deref()
            .and_then(|canonical| member.strip_prefix(canonical).ok())
        else {
            continue;
        };
        // The package itself, and members nested in a scanned tree, are covered.
        if relative.as_os_str().is_empty() || relative.starts_with("src") {
            continue;
        }
        let member_src = under(root, &relative.join("src"));
        if member_src.is_dir() && !roots.contains(&member_src) {
            roots.push(member_src);
        }
    }
    Some(roots)
}

/// Runs `scan` over each root and concatenates the findings.
pub(super) fn scan_each(
    roots: &[PathBuf],
    scan: impl Fn(&Path) -> (usize, Vec<String>),
) -> (usize, Vec<String>) {
    let mut warnings = Vec::new();
    for root in roots {
        warnings.extend(scan(root).1);
    }
    (warnings.len(), warnings)
}

/// Keeps `src/...` (not `./src/...`) in findings for the current directory.
fn under(root: &Path, relative: &Path) -> PathBuf {
    if root == Path::new(".") {
        relative.to_path_buf()
    } else {
        root.join(relative)
    }
}

#[derive(serde::Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Package {
    id: String,
    manifest_path: PathBuf,
}

/// Canonical directories of the workspace members, sorted.
fn workspace_member_directories(root: &Path) -> Vec<PathBuf> {
    let Ok(output) = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(metadata) = serde_json::from_slice::<Metadata>(&output.stdout) else {
        return Vec::new();
    };
    let mut directories = metadata
        .packages
        .into_iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .filter_map(|package| package.manifest_path.parent()?.canonicalize().ok())
        .collect::<Vec<_>>();
    directories.sort();
    directories.dedup();
    directories
}

/// Rejects anything but `RUSTSEC-YYYY-NNNN` advisory exceptions.
pub(super) fn validate_audit_ignores(
    audit_ignores: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    for advisory in audit_ignores {
        let bytes = advisory.as_bytes();
        let valid = bytes.len() == 17
            && bytes.starts_with(b"RUSTSEC-")
            && bytes[8..12].iter().all(u8::is_ascii_digit)
            && bytes[12] == b'-'
            && bytes[13..].iter().all(u8::is_ascii_digit);
        if !valid {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid --audit-ignore value '{advisory}'; expected RUSTSEC-YYYY-NNNN"),
            )
            .into());
        }
    }
    Ok(())
}

/// The lockfile `cargo audit` must read for `root`: its own `Cargo.lock`, or
/// for a workspace member the one in the workspace root that `cargo metadata`
/// (through `program`) reports. `None` leaves the choice to `cargo audit`.
pub(super) fn audit_lockfile(program: &std::ffi::OsStr, root: &Path) -> Option<PathBuf> {
    #[derive(serde::Deserialize)]
    struct Workspace {
        workspace_root: PathBuf,
    }

    let local = root.join("Cargo.lock");
    if local.is_file() {
        return Some(local);
    }
    let output = Command::new(program)
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let workspace = serde_json::from_slice::<Workspace>(&output.stdout).ok()?;
    Some(workspace.workspace_root.join("Cargo.lock")).filter(|lockfile| lockfile.is_file())
}

/// Runs `cargo audit` (through `program`, normally `cargo`) in `root` with the
/// governed exceptions, on the lockfile [`audit_lockfile`] finds. An
/// unavailable tool is `NOT CHECKED`; a run that exits non-zero, including
/// one that reports advisories, is `ERROR`.
pub(super) fn cargo_audit_status(
    program: &std::ffi::OsStr,
    root: &Path,
    audit_ignores: &[String],
) -> EvidenceStatus {
    let available = Command::new(program)
        .args(["audit", "--version"])
        .current_dir(root)
        .output()
        .is_ok_and(|tool| tool.status.success());
    if !available {
        return EvidenceStatus::NotChecked("cargo-audit is unavailable");
    }
    let mut audit = Command::new(program);
    audit
        .args(cargo_audit_arguments(audit_ignores))
        .current_dir(root);
    // A workspace member has no `Cargo.lock` of its own.
    if let Some(lockfile) = audit_lockfile(program, root) {
        audit.arg("--file").arg(lockfile);
    }
    match audit.output() {
        Ok(out) if out.status.success() && audit_ignores.is_empty() => EvidenceStatus::NoFindings,
        Ok(out) if out.status.success() => {
            EvidenceStatus::NoFindingsOutsideExceptions(audit_ignores.to_vec())
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            EvidenceStatus::Error(if stderr.is_empty() {
                format!("cargo-audit exited with status {}", out.status)
            } else {
                stderr
            })
        }
        Err(error) => EvidenceStatus::Error(error.to_string()),
    }
}

pub(super) fn cargo_audit_arguments(audit_ignores: &[String]) -> Vec<String> {
    let mut arguments = vec!["audit".to_string()];
    for advisory in audit_ignores {
        arguments.push("--ignore".to_string());
        arguments.push(advisory.clone());
    }
    arguments
}

/// Recursively scans Rust source files for `unsafe` blocks, functions, or implementations.
///
/// Symlinked files and directories are not followed; a walk that reaches its
/// bound adds a finding instead of reporting a clean scan.
pub fn scan_unsafe_code(src_dir: &Path) -> (usize, Vec<String>) {
    let sources = rust_sources(src_dir);
    let mut warnings = Vec::new();
    if let Some(reason) = sources.incomplete {
        warnings.push(super::audit::incomplete_walk_warning(
            src_dir,
            &reason,
            "source files",
        ));
    }
    for path in sources.files {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }
            if trimmed.contains("unsafe {")
                || trimmed.contains("unsafe fn")
                || trimmed.contains("unsafe impl")
                || trimmed.starts_with("unsafe ")
            {
                let msg = format!(
                    "File '{}:{}': Unsafe Rust detected: `{}`",
                    path.display(),
                    line_idx + 1,
                    trimmed
                );
                if !warnings.contains(&msg) {
                    warnings.push(msg);
                }
            }
        }
    }

    (warnings.len(), warnings)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    #[test]
    fn workspace_member_sources_are_scanned_beside_the_root_package() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path();
        write(
            &root.join("Cargo.toml"),
            "[package]\nname = \"shop\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\nmembers = [\"crates/http\"]\n",
        );
        write(&root.join("src/main.rs"), "fn main() {}\n");
        write(
            &root.join("crates/http/Cargo.toml"),
            "[package]\nname = \"http\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(
            &root.join("crates/http/src/lib.rs"),
            "pub fn routes() { get(\"/orders/{id}\" => show); }\n",
        );

        let roots = package_source_roots(root).expect("package root");
        assert_eq!(roots, vec![root.join("src"), root.join("crates/http/src")]);
        // The old audit scanned only `src` and reported this route as clean.
        let (count, warnings) =
            scan_each(&roots, crate::generators::audit::scan_idor_vulnerabilities);
        assert_eq!(count, 1, "{warnings:?}");
        assert!(warnings[0].contains("crates/http/src/lib.rs"));
    }

    #[test]
    fn a_workspace_member_audits_the_workspace_lockfile() {
        let project = tempfile::tempdir().unwrap();
        let root = project.path().canonicalize().unwrap();
        write(
            &root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"blog\"]\nresolver = \"3\"\n",
        );
        write(
            &root.join("blog/Cargo.toml"),
            "[package]\nname = \"blog\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&root.join("blog/src/main.rs"), "fn main() {}\n");
        write(&root.join("Cargo.lock"), "version = 4\n");

        let cargo = std::ffi::OsStr::new("cargo");
        // The member has no `Cargo.lock`; `cargo audit` there used to fail.
        assert_eq!(
            audit_lockfile(cargo, &root.join("blog")),
            Some(root.join("Cargo.lock"))
        );
        assert_eq!(audit_lockfile(cargo, &root), Some(root.join("Cargo.lock")));
        let outside = tempfile::tempdir().unwrap();
        assert_eq!(audit_lockfile(cargo, outside.path()), None);
    }

    #[cfg(unix)]
    #[test]
    fn cargo_audit_receives_the_workspace_lockfile() {
        use std::os::unix::fs::PermissionsExt as _;
        let project = tempfile::tempdir().unwrap();
        let root = project.path();
        write(&root.join("Cargo.lock"), "version = 4\n");
        let member = root.join("blog");
        fs::create_dir_all(&member).unwrap();
        let lockfile = root.join("Cargo.lock");
        // Succeeds only when `--file <workspace>/Cargo.lock` is passed.
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = metadata ]; then printf '{{\"workspace_root\":\"%s\"}}' '{root}'; exit 0; fi\nif [ \"$2\" = --version ]; then exit 0; fi\nprevious=\nfor argument in \"$@\"; do\n  if [ \"$previous\" = --file ] && [ \"$argument\" = '{lock}' ]; then exit 0; fi\n  previous=$argument\ndone\necho 'error: Cargo.lock not found' >&2\nexit 1\n",
            root = root.display(),
            lock = lockfile.display(),
        );
        let cargo = project.path().join("fake-cargo");
        fs::write(&cargo, script).unwrap();
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();

        let status = cargo_audit_status(cargo.as_os_str(), &member, &[]);
        assert_eq!(status, EvidenceStatus::NoFindings);
    }

    #[test]
    fn a_directory_without_src_is_not_a_package_root() {
        let project = tempfile::tempdir().unwrap();
        assert!(package_source_roots(project.path()).is_none());
        assert_eq!(under(Path::new("."), Path::new("src")), Path::new("src"));
    }
}
