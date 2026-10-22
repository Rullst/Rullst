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
        r#"# Dependency cache: this stage keeps only the manifests, Cargo.lock and the
# toolchain file, with `fn main() {{}}` standing in for each target and build
# script, so the dependency layer of the builder is rebuilt only when those
# files, or the set of targets, change.
FROM rust:1.99.1-slim-bookworm AS planner
WORKDIR /app
COPY . .
RUN set -eu; out=/skeleton; \
    find . -path ./target -prune -o -type f \( -name Cargo.toml -o -name Cargo.lock \
        -o -name rust-toolchain -o -name rust-toolchain.toml \) -print \
    | while IFS= read -r file; do \
        mkdir -p "$out/$(dirname "$file")"; cp "$file" "$out/$file"; \
        [ "$(basename "$file")" = Cargo.toml ] || continue; \
        dir=$(dirname "$file"); \
        for target in src/main.rs src/lib.rs build.rs \
            $(cd "$dir" && find src/bin -name '*.rs' 2>/dev/null) \
            $(sed -nE 's/^[[:space:]]*(path|build)[[:space:]]*=[[:space:]]*"([^"]*\.rs)".*/\2/p' "$file"); do \
            if [ -f "$dir/$target" ]; then \
                mkdir -p "$out/$dir/$(dirname "$target")"; \
                echo 'fn main() {{}}' > "$out/$dir/$target"; \
            fi; \
        done; \
    done

FROM rust:1.99.1-slim-bookworm AS builder
WORKDIR /app
COPY --from=planner /skeleton/ ./
# Generate and commit Cargo.lock before building this application image.
RUN cargo build --release --locked
COPY . .
# The real sources replace the stand-ins; touching them makes Cargo rebuild the
# application's own packages whatever modification time the copy gave them.
RUN find . -path ./target -prune -o -type f -name '*.rs' -exec touch {{}} + \
    && cargo build --release --locked
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
        assert!(dockerfile.contains("FROM rust:1.99.1-slim-bookworm AS builder"));
        // Dependencies build from the planner's skeleton before the sources
        // are copied, and every build keeps `--locked`.
        let builder = dockerfile
            .split_once("FROM rust:1.99.1-slim-bookworm AS builder\n")
            .expect("builder stage")
            .1;
        assert!(
            builder.starts_with(
                "WORKDIR /app\nCOPY --from=planner /skeleton/ ./\n# Generate and commit Cargo.lock before building this application image.\nRUN cargo build --release --locked\nCOPY . .\n"
            ),
            "{builder}"
        );
        assert!(builder.contains(
            "RUN find . -path ./target -prune -o -type f -name '*.rs' -exec touch {} + \\\n    && cargo build --release --locked\n"
        ));
        assert!(dockerfile.starts_with("# Dependency cache:"));
        assert!(dockerfile.contains("FROM rust:1.99.1-slim-bookworm AS planner\nWORKDIR /app\nCOPY . .\nRUN set -eu; out=/skeleton;"));
        assert_eq!(dockerfile.matches("cargo build").count(), 2);
        assert_eq!(
            dockerfile.matches("cargo build --release --locked").count(),
            2
        );
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

    /// Runs the planner stage's script on a workspace with a member, build
    /// script, extra binaries and a custom target path, writing the skeleton
    /// to a temporary directory instead of `/skeleton`.
    #[cfg(unix)]
    #[test]
    fn the_planner_keeps_manifests_and_stands_in_for_every_target() {
        let root = tempfile::tempdir().expect("temporary project");
        let out = tempfile::tempdir().expect("skeleton directory");
        let project = root.path();
        let write = |path: &str, text: &str| {
            let path = project.join(path);
            fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            fs::write(path, text).expect("fixture file");
        };
        write(
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n\n[[bin]]\nname = \"extra\"\npath = \"tools/extra.rs\"\n\n[dependencies]\nshared = { path = \"crates/shared\" }\n\n[workspace]\nmembers = [\"crates/*\"]\n",
        );
        write("Cargo.lock", "version = 4\n");
        write("rust-toolchain.toml", "[toolchain]\nchannel = \"1.99.1\"\n");
        write(
            "build.rs",
            "fn main() { println!(\"cargo:rerun-if-changed=build.rs\"); }\n",
        );
        write("src/main.rs", "mod routes;\nfn main() { routes::run(); }\n");
        write("src/routes.rs", "pub fn run() {}\n");
        write("src/bin/tool.rs", "fn main() { println!(\"tool\"); }\n");
        write("tools/extra.rs", "fn main() {}\n");
        write(
            "crates/shared/Cargo.toml",
            "[package]\nname = \"shared\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write("crates/shared/src/lib.rs", "pub mod util;\n");
        write("crates/shared/src/util.rs", "pub fn helper() {}\n");
        write("target/release/build/stale/Cargo.toml", "[package]\n");

        write_docker_files(project, "demo", None, None).expect("Docker files");
        let dockerfile = fs::read_to_string(project.join("Dockerfile")).expect("Dockerfile");
        let script = dockerfile
            .split_once("RUN ")
            .and_then(|(_, rest)| rest.split_once("\n\nFROM "))
            .expect("planner script")
            .0
            .replace("out=/skeleton", &format!("out='{}'", out.path().display()));
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .current_dir(project)
            .status()
            .expect("run the planner script");
        assert!(status.success(), "{script}");

        let skeleton = out.path();
        for (path, text) in [
            ("Cargo.lock", "version = 4\n"),
            ("rust-toolchain.toml", "[toolchain]\nchannel = \"1.99.1\"\n"),
            ("build.rs", "fn main() {}\n"),
            ("src/main.rs", "fn main() {}\n"),
            ("src/bin/tool.rs", "fn main() {}\n"),
            ("tools/extra.rs", "fn main() {}\n"),
            ("crates/shared/src/lib.rs", "fn main() {}\n"),
        ] {
            assert_eq!(
                fs::read_to_string(skeleton.join(path)).expect(path),
                text,
                "{path}"
            );
        }
        assert_eq!(
            fs::read(skeleton.join("Cargo.toml")).expect("root manifest"),
            fs::read(project.join("Cargo.toml")).expect("fixture manifest")
        );
        assert!(skeleton.join("crates/shared/Cargo.toml").is_file());
        for absent in [
            "src/routes.rs",
            "crates/shared/src/util.rs",
            "target",
            "Dockerfile",
        ] {
            assert!(!skeleton.join(absent).exists(), "{absent}");
        }
        // Cargo reads the skeleton as the same workspace and targets.
        let metadata = std::process::Command::new(env!("CARGO"))
            .args([
                "metadata",
                "--no-deps",
                "--offline",
                "--format-version",
                "1",
            ])
            .current_dir(skeleton)
            .output()
            .expect("cargo metadata");
        assert!(
            metadata.status.success(),
            "{}",
            String::from_utf8_lossy(&metadata.stderr)
        );
        let metadata = String::from_utf8_lossy(&metadata.stdout);
        for target in [
            "\"name\":\"extra\"",
            "\"name\":\"tool\"",
            "\"name\":\"shared\"",
        ] {
            assert!(metadata.contains(target), "{target}");
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
