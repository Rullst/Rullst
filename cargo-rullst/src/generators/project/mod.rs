// cargo-rullst/src/generators/project/mod.rs — Root of project generator module (< 200 lines).

pub mod cargo_toml;
mod command;
pub(crate) mod create;
mod docker;
pub mod env_config;
mod next_steps;
pub mod wizard;

use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};

pub(crate) use command::{new_command, run_dry_run};
pub use docker::generate_docker_files;
pub use env_config::{generate_buildah_script, generate_nix_files};
pub use wizard::{PolyglotIntegration, ProjectWizardOptions, run_project_wizard};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectScaffoldOptions {
    pub api: bool,
    pub docker: bool,
    pub buildah: bool,
    pub nix: bool,
    pub use_defaults: bool,
    pub turso: bool,
    pub mongodb: bool,
    pub duckdb: bool,
    pub surrealdb: bool,
    pub qdrant: bool,
    pub database: Option<&'static str>,
    pub no_database: bool,
    pub hot_reload: bool,
    pub wants_ai: bool,
    pub wants_redis: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIdentity {
    destination_path: PathBuf,
    package_name: String,
    module_name: String,
}

impl ProjectIdentity {
    pub fn from_destination(destination: impl AsRef<str>) -> Result<Self, IoError> {
        let raw = destination.as_ref().trim();
        let trimmed = raw.trim_end_matches(['/', '\\']);
        let package_name = trimmed
            .rsplit(['/', '\\'])
            .next()
            .filter(|name| !name.is_empty() && *name != "." && *name != "..")
            .ok_or_else(|| IoError::new(ErrorKind::InvalidInput, "invalid project destination"))?;

        let mut chars = package_name.chars();
        let valid_first = chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic());
        let valid_rest = chars.all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        });
        if !valid_first || !valid_rest {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "project package name must start with an ASCII letter and contain only letters, numbers, '-' or '_'",
            ));
        }
        let module_name = package_name.replace('-', "_");
        if is_rust_keyword(&module_name) {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "project package name normalizes to a reserved Rust keyword",
            ));
        }

        Ok(Self {
            destination_path: PathBuf::from(raw),
            package_name: package_name.to_string(),
            module_name,
        })
    }

    pub fn destination_path(&self) -> &Path {
        &self.destination_path
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn module_name(&self) -> &str {
        &self.module_name
    }
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "abstract"
            | "as"
            | "async"
            | "await"
            | "become"
            | "box"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "do"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "final"
            | "fn"
            | "for"
            | "gen"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "macro"
            | "macro_rules"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "override"
            | "priv"
            | "pub"
            | "raw"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "unsafe"
            | "unsized"
            | "use"
            | "virtual"
            | "where"
            | "while"
            | "yield"
    )
}

pub fn has_binary(name: &str) -> bool {
    if name
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_')
    {
        return false;
    }
    let cmd = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(cmd)
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn generate_secure_app_key() -> String {
    use rand::RngExt;
    let mut key = String::new();
    let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    for _ in 0..32 {
        let idx = rng.random_range(0..chars.len());
        key.push(chars[idx] as char);
    }
    key
}

pub fn create_new_project_with_options(
    name_arg: Option<&str>,
    options: ProjectScaffoldOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    create_new_project_with_cli_options(name_arg, options, None, false)
}

pub(crate) fn create_new_project_with_cli_options(
    name_arg: Option<&str>,
    options: ProjectScaffoldOptions,
    blueprint_override: Option<usize>,
    skip_initial_migration: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    create::run_new(&wizard::NewProjectRequest {
        name: name_arg,
        options,
        blueprint: blueprint_override,
        skip_initial_migration,
        dry_run: false,
    })
}

#[deprecated(
    since = "12.0.0",
    note = "use create_new_project_with_options to avoid positional flag mixups"
)]
#[allow(clippy::too_many_arguments)]
pub fn create_new_project(
    name_arg: Option<&str>,
    api: bool,
    docker: bool,
    nix: bool,
    buildah: bool,
    use_defaults: bool,
    turso: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    create_new_project_with_options(
        name_arg,
        ProjectScaffoldOptions {
            api,
            docker,
            buildah,
            nix,
            use_defaults,
            turso,
            mongodb: false,
            duckdb: false,
            surrealdb: false,
            qdrant: false,
            database: None,
            no_database: false,
            hot_reload: false,
            wants_ai: false,
            wants_redis: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_binary_validation() {
        assert!(!has_binary("invalid;binary"));
        assert!(!has_binary("binary with space"));
        assert!(!has_binary("cmd|pipe"));
        assert!(!has_binary("non_existent_binary_xyz_12345"));
        assert!(has_binary("cargo"));
    }

    #[test]
    fn test_generate_secure_app_key() {
        let key1 = generate_secure_app_key();
        let key2 = generate_secure_app_key();
        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
        assert_ne!(key1, key2);
        assert!(key1.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn project_identity_separates_destination_package_and_module() {
        let unix = ProjectIdentity::from_destination("../dummy_test").unwrap();
        assert_eq!(unix.destination_path(), Path::new("../dummy_test"));
        assert_eq!(unix.package_name(), "dummy_test");
        assert_eq!(unix.module_name(), "dummy_test");

        let hyphenated = ProjectIdentity::from_destination("../dummy-test").unwrap();
        assert_eq!(hyphenated.destination_path(), Path::new("../dummy-test"));
        assert_eq!(hyphenated.package_name(), "dummy-test");
        assert_eq!(hyphenated.module_name(), "dummy_test");

        let windows = ProjectIdentity::from_destination(r"..\dummy_test").unwrap();
        assert_eq!(windows.destination_path(), Path::new(r"..\dummy_test"));
        assert_eq!(windows.package_name(), "dummy_test");
        assert_eq!(windows.module_name(), "dummy_test");
    }

    #[test]
    fn project_identity_rejects_invalid_package_basename() {
        assert!(ProjectIdentity::from_destination("../123-app").is_err());
        assert!(ProjectIdentity::from_destination("../bad name").is_err());
        assert!(ProjectIdentity::from_destination("../").is_err());
        assert!(ProjectIdentity::from_destination("../crate").is_err());
    }
}
