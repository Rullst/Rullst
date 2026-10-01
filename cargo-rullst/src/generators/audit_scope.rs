//! Which source trees the project-source audit scans cover.

use std::path::{Path, PathBuf};
use std::process::Command;

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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

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
    fn a_directory_without_src_is_not_a_package_root() {
        let project = tempfile::tempdir().unwrap();
        assert!(package_source_roots(project.path()).is_none());
        assert_eq!(under(Path::new("."), Path::new("src")), Path::new("src"));
    }
}
