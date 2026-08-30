// cargo-rullst/src/generators/project/mod.rs — Root of project generator module (< 200 lines).

pub mod cargo_toml;
pub mod env_config;
pub mod wizard;

use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::{Path, PathBuf};

use crate::blueprints::BLANK_BLUEPRINT_ID;

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
    create_new_project_with_cli_options(name_arg, options, None, false, None)
}

pub(crate) fn create_new_project_with_cli_options(
    name_arg: Option<&str>,
    options: ProjectScaffoldOptions,
    blueprint_override: Option<usize>,
    skip_initial_migration: bool,
    lms_modules: Option<&[crate::blueprints::lms::LmsModule]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut requested_integrations = Vec::new();
    if options.turso || options.database == Some("Turso") {
        requested_integrations.push(PolyglotIntegration::Turso);
    }
    if options.mongodb {
        requested_integrations.push(PolyglotIntegration::MongoDb);
    }
    if options.duckdb {
        requested_integrations.push(PolyglotIntegration::DuckDb);
    }
    if options.surrealdb {
        requested_integrations.push(PolyglotIntegration::SurrealDb);
    }
    if options.qdrant {
        requested_integrations.push(PolyglotIntegration::Qdrant);
    }
    let wizard_opts = wizard::run_project_wizard_with_blueprint(
        name_arg,
        options.api,
        options.use_defaults,
        &requested_integrations,
        options.database,
        blueprint_override,
    )?;

    let identity = ProjectIdentity::from_destination(&wizard_opts.name)?;
    let project_name = identity.package_name();
    let project_name_safe = identity.module_name();
    let api = wizard_opts.api;
    let mut db_needed = wizard_opts.db_needed;
    let db_provider = wizard_opts.db_provider.clone();
    let hot_reload = wizard_opts.hot_reload;
    let blueprint_selection = wizard_opts.blueprint_selection;
    let wants_ai = wizard_opts.wants_ai;
    let wants_redis = wizard_opts.wants_redis;
    let polyglot_integrations = wizard_opts.polyglot_integrations.clone();

    if let Some(modules) = lms_modules {
        if blueprint_selection != crate::blueprints::LMS_BLUEPRINT_ID {
            return Err(IoError::new(
                ErrorKind::InvalidInput,
                "LMS modules may only be selected with the LMS blueprint",
            )
            .into());
        }
        crate::blueprints::lms::validate_module_selection(modules, hot_reload)?;
    }

    if blueprint_selection != BLANK_BLUEPRINT_ID {
        db_needed = true;
    }

    if db_provider == "Turso" && blueprint_selection != BLANK_BLUEPRINT_ID {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            "Turso-primary currently requires the blank starter while the SQLx-specific blueprints are being ported",
        )
        .into());
    }

    let path = identity.destination_path();
    if path.exists() {
        return Err(IoError::new(
            ErrorKind::AlreadyExists,
            format!("directory '{}' already exists", path.display()),
        )
        .into());
    }

    fs::create_dir_all(path)?;
    let current_dir = std::env::current_dir()?;

    let cargo_toml_content = cargo_toml::build_cargo_toml(
        project_name,
        hot_reload,
        db_needed,
        &db_provider,
        &polyglot_integrations,
        wants_ai,
        wants_redis,
        blueprint_selection,
        &wizard_opts.frontend_engine,
        &current_dir,
    )?;
    fs::write(path.join("Cargo.toml"), cargo_toml_content)?;

    let app_key = generate_secure_app_key();
    env_config::generate_env_and_configs(
        path,
        db_needed,
        &db_provider,
        &polyglot_integrations,
        blueprint_selection,
        &app_key,
    )?;

    // Apply Blueprint templates
    crate::blueprints::apply_with_lms_modules(
        blueprint_selection,
        path,
        project_name,
        project_name_safe,
        api,
        hot_reload,
        db_needed,
        &wizard_opts.orm_pattern,
        &wizard_opts.frontend_engine,
        lms_modules,
    )?;

    if options.docker || options.buildah {
        generate_docker_files(path, project_name, Some(&db_provider), Some(wants_redis))?;
    }

    if options.nix {
        env_config::generate_nix_files(path)?;
    }

    if db_needed && !skip_initial_migration {
        println!("\n{}", "📦 Bootstrapping Database...".cyan().bold());
        let migrate_success = crate::ui::components::with_spinner(
            "Running initial migrations (this may take a moment to compile)...",
            || {
                std::process::Command::new("cargo")
                    .arg("run")
                    .arg("-q")
                    .arg("--")
                    .arg("db:migrate")
                    .current_dir(path)
                    .output()
                    .map(|s| s.status.success())
                    .unwrap_or(false)
            },
        );

        if migrate_success {
            println!("{}", "  ✅ Database tables created successfully.".green());
        } else {
            println!(
                "{}",
                "  ⚠️ Warning: Failed to run initial database migrations.".yellow()
            );
        }
    }

    if options.buildah {
        env_config::generate_buildah_script(path, project_name)?;
    }

    println!(
        "{}",
        format!("✨ Project '{}' created successfully!", project_name)
            .green()
            .bold()
    );
    println!("{}", "How to run:".magenta());
    println!("{}", format!("  cd {path:?}").cyan());
    println!("{}", "  Then, choose your experience:".white().dimmed());
    println!(
        "{}",
        "    cargo rullst dash  (interactive dashboard)"
            .white()
            .bold()
    );
    println!("{}", "    cargo rullst dev   (standard output)".white());

    Ok(())
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
        },
    )
}

pub fn generate_docker_files(
    project_path: &Path,
    project_name: &str,
    db_provider_arg: Option<&str>,
    redis_arg: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🐳 Generating Docker files...".cyan().bold());
    let _db_provider = db_provider_arg.unwrap_or("Sqlite");
    let _wants_redis = redis_arg.unwrap_or(false);

    let dockerfile = format!(
        r#"FROM rust:1.97-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM docker.io/library/debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/{project_name} /app/{project_name}
CMD ["/app/{project_name}"]
"#
    );
    fs::write(project_path.join("Dockerfile"), dockerfile)?;
    println!("{}", "  ✅ Dockerfile generated.".green());
    Ok(())
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
