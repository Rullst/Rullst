//! Shared no-clobber policy for generators that write application files.
//!
//! Generators refuse to replace an existing entry (including a dangling
//! symlink) unless the command offers an explicit `--force`. A forced write
//! still refuses a symlinked target and replaces the directory entry through a
//! sibling temporary file instead of writing through it.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Fails with `AlreadyExists`, naming every planned output that already exists.
pub(crate) fn reject_existing(what: &str, paths: &[PathBuf], remedy: &str) -> io::Result<()> {
    let mut collisions = Vec::new();
    for path in paths {
        match fs::symlink_metadata(path) {
            Ok(_) => collisions.push(super::slash_path(path)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    if collisions.is_empty() {
        return Ok(());
    }
    collisions.sort();
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "refusing to overwrite existing {what}: {}{remedy}",
            collisions.join(", ")
        ),
    ))
}

/// Refuses a path whose final component is a symlink, so writes never follow it.
pub(crate) fn reject_symlink(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "refusing to write through symlinked generator output '{}'",
                super::slash_path(path)
            ),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Creates a new file; an existing entry, including a symlink, is never replaced.
pub(crate) fn write_new(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!(
                        "refusing to overwrite existing '{}'",
                        super::slash_path(path)
                    ),
                )
            } else {
                error
            }
        })?;
    if let Err(error) = file.write_all(contents) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

/// Writes `contents`, replacing an existing regular file only when `overwrite` is set.
pub(crate) fn write_output(path: &Path, contents: &[u8], overwrite: bool) -> io::Result<()> {
    reject_symlink(path)?;
    let existing = match fs::metadata(path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let Some(metadata) = existing.filter(|_| overwrite) else {
        return write_new(path, contents);
    };
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "generator output '{}' is not a regular file",
                super::slash_path(path)
            ),
        ));
    }
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

/// Lists `src/migrations` files whose name ends with one of `suffixes`.
pub(crate) fn existing_migrations(suffixes: &[String]) -> io::Result<Vec<PathBuf>> {
    let directory = Path::new("src/migrations");
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut matches = Vec::new();
    for entry in entries {
        let entry = entry?;
        if entry.file_name().to_str().is_some_and(|name| {
            suffixes
                .iter()
                .any(|suffix| name.ends_with(suffix.as_str()))
        }) {
            matches.push(entry.path());
        }
    }
    matches.sort();
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_outputs_are_reported_and_never_replaced_without_overwrite() {
        let directory = tempfile::tempdir().unwrap();
        let existing = directory.path().join("Dockerfile");
        let absent = directory.path().join("flake.nix");
        fs::write(&existing, "customized").unwrap();

        let error = reject_existing(
            "packaging files",
            &[absent.clone(), existing.clone()],
            "; rerun with --force",
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("Dockerfile"));
        assert!(!error.to_string().contains("flake.nix"));
        assert!(error.to_string().ends_with("; rerun with --force"));

        assert!(write_output(&existing, b"template", false).is_err());
        assert_eq!(fs::read_to_string(&existing).unwrap(), "customized");
        write_output(&existing, b"template", true).unwrap();
        assert_eq!(fs::read_to_string(&existing).unwrap(), "template");
        write_output(&absent, b"created", false).unwrap();
        assert_eq!(fs::read_to_string(&absent).unwrap(), "created");
    }

    /// Windows joins `k8s\deployment.yaml`; a Unix file name holding that
    /// backslash simulates it, and messages name it with `/` on every OS.
    #[cfg(unix)]
    #[test]
    fn refusals_name_outputs_with_forward_slashes() {
        let directory = tempfile::tempdir().unwrap();
        let existing = directory.path().join("k8s\\deployment.yaml");
        fs::write(&existing, "customized").unwrap();
        for error in [
            reject_existing("Kubernetes manifests", std::slice::from_ref(&existing), "")
                .unwrap_err(),
            write_new(&existing, b"template").unwrap_err(),
        ] {
            let message = error.to_string();
            assert!(message.contains("k8s/deployment.yaml"), "{message}");
            assert!(!message.contains('\\'), "{message}");
        }
        assert_eq!(fs::read_to_string(&existing).unwrap(), "customized");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_outputs_are_neither_followed_nor_replaced() {
        let directory = tempfile::tempdir().unwrap();
        let victim = directory.path().join("victim");
        fs::write(&victim, "outside").unwrap();
        let link = directory.path().join("deployment.yaml");
        std::os::unix::fs::symlink(&victim, &link).unwrap();
        let dangling = directory.path().join("ingress.yaml");
        std::os::unix::fs::symlink(directory.path().join("missing"), &dangling).unwrap();

        assert!(reject_existing("manifests", std::slice::from_ref(&dangling), "").is_err());
        for overwrite in [false, true] {
            assert!(write_output(&link, b"template", overwrite).is_err());
            assert!(write_output(&dangling, b"template", overwrite).is_err());
        }
        assert_eq!(fs::read_to_string(&victim).unwrap(), "outside");
        assert!(!directory.path().join("missing").exists());
    }

    #[cfg(unix)]
    #[test]
    fn lookup_errors_other_than_absence_are_reported() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("file");
        fs::write(&file, "not a directory").unwrap();
        let below = file.join("child");
        assert!(reject_existing("outputs", std::slice::from_ref(&below), "").is_err());
        assert!(reject_symlink(&below).is_err());
    }

    /// Runs `existing_migrations` in a child test process inside a project.
    #[test]
    fn existing_migrations_child_fixture() {
        let Some(report) = std::env::var_os("RULLST_OUTPUT_GUARD_REPORT") else {
            return;
        };
        let text = match existing_migrations(&["_create_users.rs".to_string()]) {
            Ok(paths) => format!("ok {paths:?}"),
            Err(error) => format!("err {}", error.kind()),
        };
        fs::write(report, text).unwrap();
    }

    fn existing_migrations_in(project: &Path) -> String {
        let report = project.join("report");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "generators::output_guard::tests::existing_migrations_child_fixture",
                "--nocapture",
            ])
            .current_dir(project)
            .env("RULLST_OUTPUT_GUARD_REPORT", &report)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        fs::read_to_string(report).unwrap()
    }

    #[test]
    fn existing_migrations_lists_only_matching_files_of_the_project() {
        let project = tempfile::tempdir().unwrap();
        assert_eq!(existing_migrations_in(project.path()), "ok []");
        let migrations = project.path().join("src/migrations");
        fs::create_dir_all(&migrations).unwrap();
        for name in [
            "20260101_create_users.rs",
            "20250101_create_users.rs",
            "20260102_create_posts.rs",
        ] {
            fs::write(migrations.join(name), "").unwrap();
        }
        // The listed paths use the platform's separator after the directory.
        let expected: Vec<String> = ["20250101_create_users.rs", "20260101_create_users.rs"]
            .iter()
            .map(|name| {
                std::path::Path::new("src/migrations")
                    .join(name)
                    .display()
                    .to_string()
            })
            .collect();
        assert_eq!(
            existing_migrations_in(project.path()),
            format!("ok {expected:?}")
        );
        fs::remove_dir_all(&migrations).unwrap();
        fs::write(&migrations, "not a directory").unwrap();
        assert!(existing_migrations_in(project.path()).starts_with("err "));
    }
}
