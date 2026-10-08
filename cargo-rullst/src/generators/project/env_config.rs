// cargo-rullst/src/generators/project/env_config.rs — Environment, gitignore, Nix, and Buildah configuration.

use crate::blueprints::{BLANK_BLUEPRINT_ID, BLOG_BLUEPRINT_ID, SAAS_BLUEPRINT_ID};
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
# Selects the linkers found on the machine that generated this project.
# Host-local: the generated .gitignore and .dockerignore exclude this file so
# CI runners and container builds without mold/lld do not inherit it.
# Windows uses the toolchain's supported linker and debug-information defaults.

"#,
    );

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
        // The starter selects Stripe explicitly. Chromium checks form-action
        // again after the local checkout POST's HTTP 303 handoff.
        let billing_csp = rullst_core::config::DEFAULT_CSP_TEMPLATE.replace(
            "form-action 'self'",
            "form-action 'self' https://checkout.stripe.com",
        );
        rullst_toml.push_str(&format!(
            r#"
[security]
# This exact path must also remain wrapped by rullst-capital signature verification.
csrf_signed_webhook_paths = ["/billing/webhook"]
# Matches the starter's explicit Stripe selection. Review this exact origin when
# changing providers; never use https: or a wildcard for hosted checkout.
# Validate the provider URL on the server too. CSP does not establish ownership.
csp = "{billing_csp}"
"#,
        ));
    }
    if !rullst_toml.is_empty() {
        fs::write(path.join("Rullst.toml"), rullst_toml)?;
    }

    let gitignore_content = format!(
        r#"# Rust build artifacts
/target

# Commit Cargo.lock for reproducible application/deployment builds.

# Rullst: Database
{LOCAL_DATABASE_IGNORES}
# Rullst: Environment & Secrets
.env
.env.*
!.env.example

# Rullst: host-local linker selection (mold/lld when found)
/.cargo/config.toml

# IDEs and OS files
.vscode/
.idea/
.DS_Store
"#,
        LOCAL_DATABASE_IGNORES = super::docker::LOCAL_DATABASE_IGNORES
    );
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

    // `Mail` facade sends (such as `make:mail` mailables) take this sender
    // when a message sets none. Development falls back to the log driver;
    // staging and production need an explicit MAIL_DRIVER before sending.
    let mail_template = r#"
# ── Mail ──────────────────────────────────────────────────────
# The Mail facade reads these from the process environment, then this file.
# Default sender for messages without `from`, e.g. MAIL_FROM="App <no-reply@example.com>";
# use an address your mail provider has verified.
MAIL_FROM=
# Staging/production must select a driver before sending mail, e.g.:
# MAIL_DRIVER=resend
"#;
    env_content.push_str(mail_template);
    env_example_content.push_str(mail_template);

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

    if blueprint_selection == BLOG_BLUEPRINT_ID {
        let origin_template = r#"
# ── Public origin ──
# Canonical HTTPS origin (no path) for the absolute URLs in robots.txt and
# sitemap.xml, e.g. https://blog.example.com. Unset, neither lists a URL.
RULLST_PUBLIC_ORIGIN=
"#;
        env_content.push_str(origin_template);
        env_example_content.push_str(origin_template);
    }

    if blueprint_selection == SAAS_BLUEPRINT_ID {
        let billing_template = r#"
# ── Billing (required in production) ──
# Generated billing code reads these from the process environment, then this file.
BILLING_PROVIDER=stripe
# Stripe platform account ID; complete setup is documented in BILLING.md.
BILLING_ACCOUNT_ID=
# Live keys require this explicit acknowledgement after reviewing BILLING.md.
# BILLING_LIVE_ACKNOWLEDGEMENT=I_UNDERSTAND_REAL_CHARGES
# When changing providers, review Rullst.toml security.csp form-action too.
# Lemon Squeezy needs your exact reviewed store/custom checkout origin, no wildcard.
# Required for live Lemon Squeezy checkout; use your merchant's numeric store ID.
BILLING_STORE_ID=
BILLING_API_KEY=
BILLING_WEBHOOK_SECRET=
BILLING_REDIRECT_URL=http://localhost:3000/dashboard
BILLING_ALLOWED_PLAN_IDS=price_starter,price_pro
# Optional plan-gated report; configure an explicit subset before enabling access.
BILLING_REPORT_PLAN_IDS=
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

/// Appends the bearer token of the Blank JSON API starter's machine endpoints:
/// a random value to `.env` and an empty entry to `.env.example`.
pub(crate) fn append_api_token(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    const SECTION: &str = "\n# ── JSON API ──────────────────────────────────────────────────\n# Bearer token for the JSON write routes: `Authorization: Bearer <API_TOKEN>`.\n# Use 32 to 200 random characters; rotate it before deployment.\n";
    let token = Alphanumeric.sample_string(&mut rand::rng(), 48);
    for (file, value) in [(".env", token.as_str()), (".env.example", "")] {
        let mut content = fs::read_to_string(path.join(file))?;
        content.push_str(SECTION);
        content.push_str(&format!("API_TOKEN={value}\n"));
        fs::write(path.join(file), content)?;
    }
    Ok(())
}

pub fn generate_nix_files(project_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    write_nix_files(project_path)?;
    println!(
        "{}",
        "  ✅ flake.nix (Nix reproducible environment)".green()
    );
    println!("{}", "  ✅ .envrc (direnv support)".green());

    Ok(())
}

/// [`generate_nix_files`] without progress output (used by previews).
pub(crate) fn write_nix_files(project_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

pub fn generate_buildah_script(
    project_path: &Path,
    project_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    write_buildah_script(project_path, project_name)?;
    println!(
        "{}",
        "\n📦 Buildah script generated! To build an OCI image rootless:".cyan()
    );
    Ok(())
}

/// [`generate_buildah_script`] without progress output (used by previews).
pub(crate) fn write_buildah_script(
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
    // OCI repository names are lowercase; `make:k8s` references the same name.
    let image = crate::blueprints::k8s::container_name(project_name);
    let buildah_script = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

echo "🦀 Building rootless OCI image {image}:latest..."
buildah bud -f Dockerfile -t {image}:latest .
echo "✅ Build complete!"
"#
    );
    let script_path = project_path.join("build_buildah.sh");
    fs::write(&script_path, buildah_script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "env_config_tests.rs"]
mod tests;
