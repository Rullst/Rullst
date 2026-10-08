//! Container build files: the multi-stage `Dockerfile` and its `.dockerignore`.

use colored::*;
use std::fs;
use std::path::Path;

/// Local SQLite and DuckDB databases and their journals, which may hold
/// personal data. The generated `.gitignore` and `.dockerignore` share this
/// list so neither Git nor the container build context picks them up.
pub(super) const LOCAL_DATABASE_IGNORES: &str = "*.db
*.db-shm
*.db-wal
*.sqlite
*.sqlite-shm
*.sqlite-wal
*.sqlite3
*.duckdb
*.duckdb.wal
";

/// Keeps what `.gitignore` treats as local or secret out of the build context,
/// which `COPY . .` would otherwise send to (possibly remote) builders and
/// their layer cache. `.cargo/config.toml` holds the generating host's linker
/// selection (mold/lld), which the builder image does not have.
fn dockerignore_content() -> String {
    format!(
        "target
.git
coverage
# Environment, deployment secrets and host-local configuration
.env
.env.*
!.env.example
Foundry.toml
.cargo/config.toml
# Local databases
{LOCAL_DATABASE_IGNORES}"
    )
}

pub fn generate_docker_files(
    project_path: &Path,
    project_name: &str,
    db_provider_arg: Option<&str>,
    redis_arg: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🐳 Generating Docker files...".cyan().bold());
    write_docker_files(project_path, project_name, db_provider_arg, redis_arg)?;
    println!("{}", "  ✅ Dockerfile generated.".green());
    Ok(())
}

/// [`generate_docker_files`] without progress output (used by previews).
pub(crate) fn write_docker_files(
    project_path: &Path,
    project_name: &str,
    db_provider_arg: Option<&str>,
    redis_arg: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_provider = db_provider_arg;
    let _wants_redis = redis_arg.unwrap_or(false);

    let copy_static = if project_path.join("static").is_dir() {
        "COPY --chown=10001:10001 static /app/static\n"
    } else {
        ""
    };
    let copy_config = if project_path.join("Rullst.toml").is_file() {
        "COPY --chown=10001:10001 Rullst.toml /app/Rullst.toml\n"
    } else {
        ""
    };
    let sqlite_environment = if matches!(db_provider, Some("Sqlite")) {
        "ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc\n"
    } else {
        ""
    };

    let dockerfile = format!(
        r#"FROM rust:1.99.0-slim-bookworm AS builder
WORKDIR /app
COPY . .
# Generate and commit Cargo.lock before building this application image.
RUN cargo build --release --locked
# The runtime image has no shell, so its directory tree is prepared here.
RUN install --directory --owner=10001 --group=10001 /runtime/app /runtime/app/data

# Debian 12 glibc runtime (libgcc, libstdc++, CA certificates, tzdata) without
# a shell or package manager: probe `/health` and `/ready` over HTTP from the
# platform instead of a shell HEALTHCHECK.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder --chown=10001:10001 /runtime/app /app
WORKDIR /app
COPY --from=builder --chown=10001:10001 /app/target/release/{project_name} /app/{project_name}
{copy_static}{copy_config}ENV HOME=/app
ENV RULLST_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000
{sqlite_environment}USER 10001:10001
EXPOSE 3000
# Run `/app/{project_name} db:migrate` once as a deployment migration job before
# starting or replacing replicas. Do not run concurrent schema changes per replica.
CMD ["/app/{project_name}"]
"#
    );
    fs::write(project_path.join("Dockerfile"), dockerfile)?;
    let dockerignore = project_path.join(".dockerignore");
    if !dockerignore.exists() {
        fs::write(dockerignore, dockerignore_content())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_sqlite_container_is_non_root_remote_bound_and_secret_free() {
        let root = tempfile::tempdir().expect("temporary project");
        fs::create_dir(root.path().join("static")).expect("static directory");
        fs::write(root.path().join("Rullst.toml"), "[app]\n").expect("Rullst config");
        generate_docker_files(root.path(), "demo", Some("Sqlite"), Some(false))
            .expect("Docker files");
        let dockerfile =
            fs::read_to_string(root.path().join("Dockerfile")).expect("generated Dockerfile");
        assert!(dockerfile.contains("FROM rust:1.99.0-slim-bookworm AS builder"));
        // The runtime stage is distroless: no shell, no package manager, and
        // the writable `/app/data` tree comes from the builder stage.
        let runtime = dockerfile
            .split_once("FROM gcr.io/distroless/cc-debian12:nonroot\n")
            .expect("distroless runtime stage")
            .1;
        assert!(!runtime.contains("RUN "), "{runtime}");
        assert!(!dockerfile.contains("apt-get"));
        assert!(dockerfile.contains(
            "install --directory --owner=10001 --group=10001 /runtime/app /runtime/app/data"
        ));
        assert!(runtime.contains("COPY --from=builder --chown=10001:10001 /runtime/app /app\n"));
        assert!(runtime.contains("/app/target/release/demo /app/demo"));
        assert!(runtime.contains("ENV HOME=/app"));
        assert!(runtime.contains("CMD [\"/app/demo\"]"));
        assert!(dockerfile.contains("ENV RULLST_ENV=production"));
        assert!(dockerfile.contains("ENV HOST=0.0.0.0"));
        assert!(dockerfile.contains("USER 10001:10001"));
        assert!(dockerfile.contains("sqlite:///app/data/rullst.db?mode=rwc"));
        assert!(dockerfile.contains("static /app/static"));
        assert!(dockerfile.contains("Rullst.toml /app/Rullst.toml"));
        assert!(dockerfile.contains("db:migrate"));
        assert!(!dockerfile.contains("APP_KEY="));
        assert!(!dockerfile.contains("VOLUME"));
        let dockerignore =
            fs::read_to_string(root.path().join(".dockerignore")).expect("dockerignore");
        assert!(dockerignore.lines().any(|line| line == ".env"));
        assert!(dockerignore.lines().any(|line| line == "target"));
        // The development database, `.env.*` and Foundry.toml (APP_KEY) are
        // private like in `.gitignore`, so they never enter the build context.
        for private in [
            ".cargo/config.toml",
            ".env.*",
            "Foundry.toml",
            "*.sqlite",
            "*.sqlite3",
            "*.duckdb",
        ] {
            assert!(
                dockerignore.lines().any(|line| line == private),
                "{private}"
            );
        }
        assert!(dockerignore.lines().any(|line| line == "!.env.example"));
    }

    #[test]
    fn gitignore_and_dockerignore_share_the_local_database_list() {
        let root = tempfile::tempdir().expect("temporary project");
        super::super::env_config::generate_env_and_configs(
            root.path(),
            false,
            "Sqlite",
            &[crate::generators::project::PolyglotIntegration::DuckDb],
            crate::blueprints::BLANK_BLUEPRINT_ID,
            "0123456789abcdef0123456789abcdef",
        )
        .expect("environment scaffold");
        generate_docker_files(root.path(), "demo", Some("Sqlite"), Some(false))
            .expect("Docker files");
        let gitignore = fs::read_to_string(root.path().join(".gitignore")).expect("gitignore");
        let dockerignore =
            fs::read_to_string(root.path().join(".dockerignore")).expect("dockerignore");
        // `--duckdb` writes `DUCKDB_PATH=analytics.duckdb` at the project root.
        for database in ["*.duckdb", "*.duckdb.wal", "*.sqlite-shm", "*.sqlite-wal"] {
            assert!(LOCAL_DATABASE_IGNORES.lines().any(|line| line == database));
        }
        for database in LOCAL_DATABASE_IGNORES.lines() {
            assert!(gitignore.lines().any(|line| line == database), "{database}");
            assert!(
                dockerignore.lines().any(|line| line == database),
                "{database}"
            );
        }
    }

    #[test]
    fn dockerize_without_known_database_does_not_override_database_url() {
        let root = tempfile::tempdir().expect("temporary project");
        generate_docker_files(root.path(), "demo", None, None).expect("Docker files");
        let dockerfile =
            fs::read_to_string(root.path().join("Dockerfile")).expect("generated Dockerfile");
        assert!(!dockerfile.contains("ENV DATABASE_URL="));
    }
}
