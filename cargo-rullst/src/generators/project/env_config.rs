// cargo-rullst/src/generators/project/env_config.rs — Environment, gitignore, Nix, and Buildah configuration.

use crate::blueprints::{BLANK_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};
use crate::generators::project::PolyglotIntegration;
use crate::generators::project::has_binary;
use colored::*;
use rand::distr::{Alphanumeric, SampleString};
use std::fs;
use std::path::Path;

pub fn generate_env_and_configs(
    path: &Path,
    db_needed: bool,
    db_provider: &str,
    polyglot_integrations: &[PolyglotIntegration],
    blueprint_selection: usize,
    app_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let cargo_dir = path.join(".cargo");
    fs::create_dir_all(&cargo_dir)?;

    let has_mold = has_binary("mold");
    let has_lld = has_binary("lld") || has_binary("lld-link");

    let mut config_toml = String::new();
    config_toml.push_str(
        r#"# 🚀 Rullst Compiler & Linker Optimization Configuration
# Configures ultra-fast linkers for local development.

"#,
    );

    if has_lld && cfg!(windows) {
        config_toml.push_str(
            r#"[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld", "-C", "link-arg=/DEBUG:FASTLINK"]

"#,
        );
    } else {
        config_toml.push_str(
            r#"[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=/DEBUG:FASTLINK"]

"#,
        );
    }

    if has_mold && cfg!(target_os = "linux") {
        config_toml.push_str(
            r#"[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold", "-C", "split-debuginfo=unpacked"]

"#,
        );
    } else if has_lld && cfg!(target_os = "linux") {
        config_toml.push_str(
            r#"[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld", "-C", "split-debuginfo=unpacked"]

"#,
        );
    } else {
        config_toml.push_str(
            r#"[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "split-debuginfo=unpacked"]

"#,
        );
    }

    fs::write(cargo_dir.join("config.toml"), config_toml)?;

    let mut rullst_toml = String::new();
    if db_needed && db_provider != "Turso" {
        let db_url = match db_provider {
            "Postgres" => "postgres://user:password@localhost:5432/db",
            "MySQL" | "MariaDB" => "mysql://user:password@localhost:3306/db",
            _ => "sqlite://db.sqlite",
        };
        rullst_toml.push_str(&format!(
            r#"[database]
url = "{db_url}"
"#
        ));
    }
    if blueprint_selection == SAAS_BLUEPRINT_ID {
        rullst_toml.push_str(
            r#"
[security]
# This exact path must also remain wrapped by rullst-capital signature verification.
csrf_signed_webhook_paths = ["/billing/webhook"]
"#,
        );
    }
    if !rullst_toml.is_empty() {
        fs::write(path.join("Rullst.toml"), rullst_toml)?;
    }

    let gitignore_content = r#"# Rust build artifacts
/target
/Cargo.lock

# Rullst: Database
*.db
*.db-shm
*.db-wal
*.sqlite
*.sqlite3

# Rullst: Environment & Secrets
.env
.env.*
!.env.example

# IDEs and OS files
.vscode/
.idea/
.DS_Store
"#;
    fs::write(path.join(".gitignore"), gitignore_content)?;

    let db_url = match db_provider {
        "Postgres" => "postgres://user:password@localhost:5432/db".to_string(),
        "MySQL" | "MariaDB" => "mysql://user:password@localhost:3306/db".to_string(),
        _ => "sqlite://db.sqlite?mode=rwc".to_string(),
    };

    let mut env_content = format!(
        r#"# Rullst Application Environment Configuration
APP_KEY={app_key}
RULLST_ENV=development
"#,
        app_key = app_key
    );

    let mut env_example_content = r#"APP_KEY=REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY
RULLST_ENV=development
"#
    .to_string();

    if db_needed && db_provider != "Turso" {
        let db_env_str = format!(
            "\n# ── Database ──────────────────────────────────────────────────\nDATABASE_URL={}\n",
            db_url
        );
        env_content.push_str(&db_env_str);
        env_example_content.push_str(&db_env_str);
    }

    for integration in polyglot_integrations {
        let (development, example) = match integration {
            PolyglotIntegration::Turso => (
                "\n# ── Turso / libSQL edge SQL ───────────────────────────────────\nTURSO_DATABASE_URL=mock_local\nTURSO_AUTH_TOKEN=\nTURSO_OFFLINE_PATH=turso-development.db\n",
                "\n# ── Turso / libSQL edge SQL ───────────────────────────────────\nTURSO_DATABASE_URL=\nTURSO_AUTH_TOKEN=\nTURSO_OFFLINE_PATH=turso-development.db\n",
            ),
            PolyglotIntegration::MongoDb => (
                "\n# ── MongoDB document store ─────────────────────────────────────\nMONGODB_URL=mock_local\nMONGODB_DATABASE=rullst_development\n",
                "\n# ── MongoDB document store ─────────────────────────────────────\nMONGODB_URL=\nMONGODB_DATABASE=\n",
            ),
            PolyglotIntegration::DuckDb => (
                "\n# ── DuckDB analytics ───────────────────────────────────────────\nDUCKDB_PATH=analytics.duckdb\n",
                "\n# ── DuckDB analytics ───────────────────────────────────────────\nDUCKDB_PATH=analytics.duckdb\n",
            ),
            PolyglotIntegration::SurrealDb => (
                "\n# ── SurrealDB document and graph store ─────────────────────────\nSURREALDB_URL=mock_local\nSURREALDB_NAMESPACE=rullst\nSURREALDB_DATABASE=development\nSURREALDB_TOKEN=\n",
                "\n# ── SurrealDB document and graph store ─────────────────────────\nSURREALDB_URL=\nSURREALDB_NAMESPACE=\nSURREALDB_DATABASE=\nSURREALDB_TOKEN=\n",
            ),
            PolyglotIntegration::Qdrant => (
                "\n# ── Qdrant dense-vector store ──────────────────────────────────\nQDRANT_URL=mock_local\nQDRANT_API_KEY=\n",
                "\n# ── Qdrant dense-vector store ──────────────────────────────────\nQDRANT_URL=\nQDRANT_API_KEY=\n",
            ),
        };
        env_content.push_str(development);
        env_example_content.push_str(example);
    }

    if blueprint_selection != BLANK_BLUEPRINT_ID {
        let mut rng = rand::rng();
        let nexus_username = format!("nexus_{}", Alphanumeric.sample_string(&mut rng, 12));
        let nexus_password = Alphanumeric.sample_string(&mut rng, 32);
        env_content.push_str(&format!(
            "\n# ── Nexus Admin (generated uniquely; rotate before deployment) ──\nNEXUS_ADMIN_USERNAME={nexus_username}\nNEXUS_ADMIN_PASSWORD={nexus_password}\n"
        ));
        env_example_content.push_str(
            "\n# ── Nexus Admin (required; use unique values, password >= 16 chars) ──\nNEXUS_ADMIN_USERNAME=\nNEXUS_ADMIN_PASSWORD=\n",
        );
    }

    if blueprint_selection == SAAS_BLUEPRINT_ID {
        let billing_template = r#"
# ── Billing (required in production) ──
BILLING_PROVIDER=stripe
BILLING_API_KEY=
BILLING_WEBHOOK_SECRET=
BILLING_REDIRECT_URL=http://localhost:3000/dashboard
BILLING_ALLOWED_PLAN_IDS=price_starter,price_pro
"#;
        env_content.push_str(billing_template);
        env_example_content.push_str(billing_template);
    }

    fs::write(path.join(".env"), &env_content)?;
    fs::write(path.join(".env.example"), &env_example_content)?;

    if db_provider == "Sqlite" {
        fs::write(path.join("rullst.db"), "")?;
    }

    Ok(())
}

pub fn generate_nix_files(project_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let flake_nix = r#"{
  description = "A Rullst Application";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustVersion = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
        craneLib = crane.mkLib pkgs;
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = [
            rustVersion
            pkgs.pkg-config
            pkgs.openssl
            pkgs.sqlite
          ];
          shellHook = ''
            echo "🦀 Welcome to the Rullst Nix Development Environment 🦀"
          '';
        };
      }
    );
}
"#;

    let envrc = r#"use flake
"#;

    fs::write(project_path.join("flake.nix"), flake_nix)?;
    fs::write(project_path.join(".envrc"), envrc)?;

    println!(
        "{}",
        "  ✅ flake.nix (Nix reproducible environment)".green()
    );
    println!("{}", "  ✅ .envrc (direnv support)".green());

    Ok(())
}

pub fn generate_buildah_script(
    project_path: &Path,
    project_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if project_name.is_empty()
        || !project_name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Buildah image name contains unsupported characters",
        )
        .into());
    }
    let buildah_script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

echo "🦀 Building rootless OCI image for {}..."
buildah bud -f Dockerfile -t {}:latest .
echo "✅ Build complete!"
"#,
        project_name, project_name
    );
    let script_path = project_path.join("build_buildah.sh");
    fs::write(&script_path, buildah_script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }
    println!(
        "{}",
        "\n📦 Buildah script generated! To build an OCI image rootless:".cyan()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nix_and_buildah_generators_emit_distinct_artifacts() {
        let root =
            std::env::temp_dir().join(format!("rullst-container-flags-{}", rand::random::<u64>()));
        fs::create_dir_all(&root).expect("temporary project");

        generate_nix_files(&root).expect("Nix files");
        assert!(root.join("flake.nix").is_file());
        assert!(root.join(".envrc").is_file());
        assert!(!root.join("build_buildah.sh").exists());

        generate_buildah_script(&root, "demo-app").expect("Buildah script");
        let script = fs::read_to_string(root.join("build_buildah.sh")).expect("Buildah source");
        assert!(script.contains("buildah bud"));
        assert!(script.contains("demo-app:latest"));

        fs::remove_dir_all(root).expect("temporary project cleanup");
    }

    #[test]
    fn generated_environment_uses_the_canonical_rullst_name() {
        let root = std::env::temp_dir().join(format!("rullst-env-name-{}", rand::random::<u64>()));
        fs::create_dir_all(&root).expect("temporary project");

        generate_env_and_configs(
            &root,
            false,
            "Sqlite",
            &[],
            BLANK_BLUEPRINT_ID,
            "0123456789abcdef0123456789abcdef",
        )
        .expect("environment scaffold");

        for filename in [".env", ".env.example"] {
            let generated =
                fs::read_to_string(root.join(filename)).expect("generated environment file");
            assert!(generated.contains("RULLST_ENV=development"));
            assert!(!generated.contains("APP_ENV="));
        }

        fs::remove_dir_all(root).expect("temporary project cleanup");
    }

    #[test]
    fn selected_persistence_integrations_get_offline_safe_development_values() {
        let root =
            std::env::temp_dir().join(format!("rullst-polyglot-env-{}", rand::random::<u64>()));
        fs::create_dir_all(&root).expect("temporary project");

        generate_env_and_configs(
            &root,
            true,
            "MariaDB",
            &[
                PolyglotIntegration::Turso,
                PolyglotIntegration::MongoDb,
                PolyglotIntegration::DuckDb,
                PolyglotIntegration::SurrealDb,
                PolyglotIntegration::Qdrant,
            ],
            BLANK_BLUEPRINT_ID,
            "0123456789abcdef0123456789abcdef",
        )
        .expect("polyglot environment scaffold");

        let development = fs::read_to_string(root.join(".env")).expect("development env");
        let example = fs::read_to_string(root.join(".env.example")).expect("example env");
        assert!(development.contains("DATABASE_URL=mysql://"));
        assert!(development.contains("TURSO_DATABASE_URL=mock_local"));
        assert!(development.contains("MONGODB_URL=mock_local"));
        assert!(development.contains("DUCKDB_PATH=analytics.duckdb"));
        assert!(development.contains("SURREALDB_URL=mock_local"));
        assert!(development.contains("QDRANT_URL=mock_local"));
        assert!(example.contains("TURSO_DATABASE_URL=\n"));
        assert!(example.contains("MONGODB_URL=\n"));
        assert!(example.contains("SURREALDB_URL=\n"));
        assert!(example.contains("QDRANT_URL=\n"));

        fs::remove_dir_all(root).expect("temporary project cleanup");
    }

    #[test]
    fn turso_primary_has_no_fictitious_sqlx_database_url() {
        let root = std::env::temp_dir().join(format!("rullst-turso-env-{}", rand::random::<u64>()));
        fs::create_dir_all(&root).expect("temporary project");

        generate_env_and_configs(
            &root,
            true,
            "Turso",
            &[PolyglotIntegration::Turso],
            BLANK_BLUEPRINT_ID,
            "0123456789abcdef0123456789abcdef",
        )
        .expect("Turso-primary environment scaffold");

        let development = fs::read_to_string(root.join(".env")).expect("development env");
        assert!(
            !development
                .lines()
                .any(|line| line.starts_with("DATABASE_URL="))
        );
        assert!(development.contains("TURSO_DATABASE_URL=mock_local"));
        assert!(development.contains("TURSO_OFFLINE_PATH=turso-development.db"));
        assert!(!root.join("Rullst.toml").exists());

        fs::remove_dir_all(root).expect("temporary project cleanup");
    }

    #[test]
    fn buildah_write_failures_are_propagated() {
        let missing_parent = std::env::temp_dir()
            .join(format!("rullst-missing-buildah-{}", rand::random::<u64>()))
            .join("project");
        assert!(generate_buildah_script(&missing_parent, "demo").is_err());
    }

    #[test]
    // TM-DEPLOY-05: validated generator inputs cannot cross into shell syntax.
    fn buildah_image_name_cannot_inject_shell_commands() {
        let root =
            std::env::temp_dir().join(format!("rullst-buildah-name-{}", rand::random::<u64>()));
        fs::create_dir_all(&root).expect("temporary project");
        assert!(generate_buildah_script(&root, "demo; touch compromised").is_err());
        assert!(!root.join("build_buildah.sh").exists());
        fs::remove_dir_all(root).expect("temporary project cleanup");
    }
}
