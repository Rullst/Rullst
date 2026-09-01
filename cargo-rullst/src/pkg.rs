//! Bounded helpers for adding and listing community crate dependencies.
//!
//! This module deliberately does not execute package code, discover a remote
//! registry, or mutate application routing. It only edits the local Cargo
//! manifest after validating the dependency name.

use colored::Colorize as _;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml_edit::{DocumentMut, Item, Table, value};

const MAX_PACKAGE_NAME_BYTES: usize = 64;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("Cargo.toml was not found at {0}")]
    MissingManifest(PathBuf),
    #[error("invalid Rullst package name `{0}`")]
    InvalidPackageName(String),
    #[error("failed to read {path}: {source}")]
    ReadManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {source}")]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: toml_edit::TomlError,
    },
    #[error("`dependencies` in {0} is not a TOML table")]
    InvalidDependenciesTable(PathBuf),
    #[error("failed to write {path}: {source}")]
    WriteManifest {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageAddOutcome {
    Added,
    AlreadyPresent,
}

/// Adds a validated `rullst-*` or `rullst_*` dependency to `Cargo.toml`.
///
/// The selected version follows the installed CLI release train. Cargo remains
/// responsible for resolving and downloading the dependency during the next
/// build. No package-owned generator or application code is executed here.
pub fn pkg_add(package_name: &str) -> Result<PackageAddOutcome, PackageError> {
    let manifest = Path::new("Cargo.toml");
    let outcome = add_to_manifest(manifest, package_name, env!("CARGO_PKG_VERSION"))?;

    match outcome {
        PackageAddOutcome::Added => {
            println!(
                "{}",
                format!("Added `{package_name}` to Cargo.toml.")
                    .green()
                    .bold()
            );
            println!("{}", "Run `cargo check` to resolve and verify it.".cyan());
        }
        PackageAddOutcome::AlreadyPresent => {
            println!(
                "{}",
                format!("`{package_name}` is already present in Cargo.toml.").yellow()
            );
        }
    }

    Ok(outcome)
}

/// Lists Rullst-prefixed entries from the manifest dependency table.
pub fn pkg_list() -> Result<Vec<String>, PackageError> {
    let manifest = Path::new("Cargo.toml");
    let packages = packages_from_manifest(manifest)?;

    println!("{}", "Rullst dependencies:".bold());
    if packages.is_empty() {
        println!("{}", "  (none found)".dimmed());
    } else {
        for package in &packages {
            println!("  • {}", package.cyan());
        }
    }

    Ok(packages)
}

fn add_to_manifest(
    manifest: &Path,
    package_name: &str,
    version: &str,
) -> Result<PackageAddOutcome, PackageError> {
    validate_package_name(package_name)?;
    let mut document = read_manifest(manifest)?;

    if !document.contains_key("dependencies") {
        document.insert("dependencies", Item::Table(Table::new()));
    }
    let dependencies = document["dependencies"]
        .as_table_mut()
        .ok_or_else(|| PackageError::InvalidDependenciesTable(manifest.to_path_buf()))?;

    if dependencies.contains_key(package_name) {
        return Ok(PackageAddOutcome::AlreadyPresent);
    }
    dependencies.insert(package_name, value(version));

    fs::write(manifest, document.to_string()).map_err(|source| PackageError::WriteManifest {
        path: manifest.to_path_buf(),
        source,
    })?;
    Ok(PackageAddOutcome::Added)
}

fn packages_from_manifest(manifest: &Path) -> Result<Vec<String>, PackageError> {
    let document = read_manifest(manifest)?;
    let Some(dependencies) = document.get("dependencies") else {
        return Ok(Vec::new());
    };
    let dependencies = dependencies
        .as_table()
        .ok_or_else(|| PackageError::InvalidDependenciesTable(manifest.to_path_buf()))?;

    let mut packages = dependencies
        .iter()
        .map(|(name, _)| name)
        .filter(|name| has_rullst_prefix(name))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    packages.sort_unstable();
    Ok(packages)
}

fn read_manifest(manifest: &Path) -> Result<DocumentMut, PackageError> {
    if !manifest.is_file() {
        return Err(PackageError::MissingManifest(manifest.to_path_buf()));
    }
    let source = fs::read_to_string(manifest).map_err(|source| PackageError::ReadManifest {
        path: manifest.to_path_buf(),
        source,
    })?;
    source
        .parse::<DocumentMut>()
        .map_err(|source| PackageError::ParseManifest {
            path: manifest.to_path_buf(),
            source,
        })
}

fn validate_package_name(package_name: &str) -> Result<(), PackageError> {
    let length_is_valid = !package_name.is_empty()
        && package_name.len() <= MAX_PACKAGE_NAME_BYTES
        && package_name.is_ascii();
    let boundary_is_valid = package_name
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && package_name
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric);
    let alphabet_is_valid = package_name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));

    if length_is_valid && boundary_is_valid && alphabet_is_valid && has_rullst_prefix(package_name)
    {
        Ok(())
    } else {
        Err(PackageError::InvalidPackageName(package_name.to_owned()))
    }
}

fn has_rullst_prefix(package_name: &str) -> bool {
    package_name.starts_with("rullst-") || package_name.starts_with("rullst_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn adds_a_validated_dependency_without_losing_existing_manifest_data() {
        let directory = tempdir().expect("temporary directory");
        let manifest = directory.path().join("Cargo.toml");
        fs::write(
            &manifest,
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nserde = \"1\"\n",
        )
        .expect("fixture manifest");

        let outcome =
            add_to_manifest(&manifest, "rullst-example", "12.0.0").expect("valid addition");
        assert_eq!(outcome, PackageAddOutcome::Added);

        let parsed = read_manifest(&manifest).expect("written manifest parses");
        assert_eq!(parsed["dependencies"]["serde"].as_str(), Some("1"));
        assert_eq!(
            parsed["dependencies"]["rullst-example"].as_str(),
            Some("12.0.0")
        );
    }

    #[test]
    fn existing_dependency_is_idempotent() {
        let directory = tempdir().expect("temporary directory");
        let manifest = directory.path().join("Cargo.toml");
        let source = "[dependencies]\nrullst-example = { version = \"11\", features = [\"x\"] }\n";
        fs::write(&manifest, source).expect("fixture manifest");

        let outcome =
            add_to_manifest(&manifest, "rullst-example", "12.0.0").expect("idempotent add");
        assert_eq!(outcome, PackageAddOutcome::AlreadyPresent);
        assert_eq!(fs::read_to_string(manifest).expect("manifest"), source);
    }

    #[test]
    fn rejects_toml_injection_and_unscoped_dependencies() {
        for invalid in [
            "serde",
            "rullst-evil\nother = \"*\"",
            "rullst/path",
            "rullst-é",
            "rullst-",
        ] {
            assert!(
                matches!(
                    validate_package_name(invalid),
                    Err(PackageError::InvalidPackageName(_))
                ),
                "{invalid:?} must be rejected"
            );
        }
    }

    #[test]
    fn lists_only_dependency_keys_with_the_rullst_prefix() {
        let directory = tempdir().expect("temporary directory");
        let manifest = directory.path().join("Cargo.toml");
        fs::write(
            &manifest,
            "[dependencies]\nserde = \"1\"\nrullst-z = \"12\"\nrullst_a = \"12\"\n",
        )
        .expect("fixture manifest");

        assert_eq!(
            packages_from_manifest(&manifest).expect("package list"),
            vec!["rullst-z".to_owned(), "rullst_a".to_owned()]
        );
    }
}
