// cargo-rullst/src/generators/project/env_config.rs — Environment, gitignore, nix, and buildah configurations (< 250 lines).

use crate::generators::project::has_binary;
use colored::*;
use std::fs;
use std::path::Path;

pub fn generate_env_and_configs(
    path: &Path,
    db_needed: bool,
    db_provider: &str,
    turso: bool,
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

    if db_needed {
        let db_url = match db_provider {
            "Postgres" => "postgres://user:password@localhost:5432/db",
            "MySQL" => "mysql://user:password@localhost:3306/db",
            "Turso" => "libsql://[your-database-id].turso.io?authToken=[your-token]",
            _ => "sqlite://db.sqlite",
        };
        let rullst_toml = format!(
            r#"[database]
url = "{db_url}"
"#
        );
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
        "MySQL" => "mysql://user:password@localhost:3306/db".to_string(),
        "Turso" => "sqlite://turso_local.db?mode=rwc".to_string(),
        _ => "sqlite://db.sqlite?mode=rwc".to_string(),
    };

    let mut env_content = format!(
        r#"# Rullst Application Environment Configuration
APP_KEY={app_key}
APP_ENV=development
"#,
        app_key = app_key
    );

    let mut env_example_content = r#"APP_KEY=REPLACE_WITH_YOUR_32_CHAR_RANDOM_KEY
APP_ENV=development
"#
    .to_string();

    if db_needed {
        let db_env_str = format!(
            "\n# ── Database ──────────────────────────────────────────────────\nDATABASE_URL={}\n",
            db_url
        );
        env_content.push_str(&db_env_str);
        env_example_content.push_str(&db_env_str);

        if turso {
            let turso_env = "\n# ── Turso Credentials ─────────────────────────\n# TURSO_DATABASE_URL=libsql://your-db-name.turso.io\n# TURSO_AUTH_TOKEN=your-auth-token\n";
            env_content.push_str(turso_env);
            env_example_content.push_str(turso_env);
        }
    }

    if blueprint_selection == 2 || blueprint_selection == 3 {
        let stripe_template = r#"
# ── Stripe Billing ──
# STRIPE_SECRET_KEY=sk_test_REPLACE_WITH_YOUR_SECRET_KEY
# STRIPE_WEBHOOK_SECRET=whsec_REPLACE_WITH_YOUR_WEBHOOK_SECRET
"#;
        env_content.push_str(stripe_template);
        env_example_content.push_str(stripe_template);
    }

    fs::write(path.join(".env"), &env_content)?;
    fs::write(path.join(".env.example"), &env_example_content)?;

    if db_provider == "Sqlite" {
        let _ = fs::write(path.join("rullst.db"), "");
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
    println!(
        "{}",
        "\n📦 Buildah script generated! To build an OCI image rootless:".cyan()
    );
    let buildah_script = format!(
        r#"#!/usr/bin/env bash
echo "🦀 Building rootless OCI image for {}..."
buildah bud -f Dockerfile -t {}:latest .
echo "✅ Build complete!"
"#,
        project_name, project_name
    );
    let script_path = project_path.join("build_buildah.sh");
    fs::write(&script_path, buildah_script).ok();
    Ok(())
}
