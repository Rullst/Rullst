//! `cargo rullst deploy`: the files that run the application on any Linux VM
//! with Docker or Podman. It writes, when absent, the Dockerfile (with its
//! dependency-cache stage), `compose.yaml` (the app and a Caddy reverse
//! proxy with automatic HTTPS) and the `Caddyfile`, and trusts that proxy in
//! `Rullst.toml`. Existing files are kept and nothing is uploaded or started.
//! Other targets (Nix, Buildah, Podman Quadlet, Kubernetes) are described in
//! the "Advanced deployment" guide.

use colored::*;
use std::fs;
use std::path::Path;

use crate::blueprints::deploy::{CADDYFILE, COMPOSE_YAML, VPS_PROXY_ADDRESS};
use crate::generators::output_guard::write_new;
use crate::generators::platform_name::package_name;

const ADVANCED_DEPLOYMENT: &str = "https://rullst.github.io/Rullst/book/advanced-deployment.html";
/// The compose file `deploy --platform vps` wrote before 13.0.0-alpha.4.
const LEGACY_COMPOSE: &str = "docker-compose.prod.yml";

/// Accepts the one deployment path; `--platform vps` (its former name) is
/// still accepted, and the removed platform targets say where to look.
fn check_platform(platform: Option<&str>) -> Result<(), String> {
    let Some(platform) = platform else {
        return Ok(());
    };
    match platform.to_lowercase().as_str() {
        "vps" => Ok(()),
        removed @ ("fly" | "fly.io" | "railway" | "render") => Err(format!(
            "`deploy --platform {removed}` was removed in 13.0.0-alpha.4. `cargo rullst deploy` writes a Dockerfile, compose.yaml and Caddyfile for any Linux VM with Docker or Podman; existing provider files stay yours. See {ADVANCED_DEPLOYMENT}"
        )),
        other => Err(format!(
            "Unknown platform '{other}'. `cargo rullst deploy` takes no platform: it writes a Dockerfile, compose.yaml and Caddyfile for any Linux VM"
        )),
    }
}

/// A Cargo package name: ASCII letters, digits, `-` and `_`. The Dockerfile
/// names the binary after it, so anything else is refused before writing.
fn safe_package_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Writes `contents` at `path` unless an entry (even a link) exists there;
/// whether it was written.
fn write_if_absent(path: &Path, contents: &str) -> Result<bool, std::io::Error> {
    if fs::symlink_metadata(path).is_ok() {
        return Ok(false);
    }
    write_new(path, contents.as_bytes())?;
    Ok(true)
}

pub fn run_deploy(platform_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    deploy_in(Path::new("."), platform_arg)
}

/// [`run_deploy`] for the project at `root`.
fn deploy_in(root: &Path, platform_arg: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    check_platform(platform_arg).map_err(std::io::Error::other)?;
    let project_name =
        package_name(&root.join("Cargo.toml")).unwrap_or_else(|| "rullst_app".to_string());
    if !safe_package_name(&project_name) {
        return Err(std::io::Error::other(format!(
            "the package name in Cargo.toml is not a valid Cargo package name ({} characters); the Dockerfile names the binary after it",
            project_name.chars().count()
        ))
        .into());
    }
    println!("{}", "🚀 Rullst deployment files".bold().cyan());

    // Before the Dockerfile scaffold, so a new Dockerfile copies Rullst.toml.
    trust_proxy(&root.join("Rullst.toml"))?;
    let database = crate::generators::project::project_database_provider(root);
    let dockerfile = root.join("Dockerfile");
    if fs::symlink_metadata(&dockerfile).is_err() {
        crate::generators::project::generate_docker_files(root, &project_name, database, None)?;
    } else if let Ok(existing) = fs::read_to_string(&dockerfile) {
        for warning in existing_dockerfile_warnings(&existing, database) {
            println!("{}", format!("  ⚠️ {warning}").yellow());
        }
    }
    for (name, contents) in [("compose.yaml", COMPOSE_YAML), ("Caddyfile", CADDYFILE)] {
        let message = if write_if_absent(&root.join(name), contents)? {
            format!("  ✅ Created {name}").green()
        } else {
            format!("  ℹ️ Existing {name} retained.").blue()
        };
        println!("{message}");
    }
    if fs::symlink_metadata(root.join(LEGACY_COMPOSE)).is_ok() {
        println!(
            "{}",
            format!("  ℹ️ compose.yaml replaces {LEGACY_COMPOSE} from earlier releases; compare them, then remove the old file.").blue()
        );
    }
    println!("{}", next_steps(&project_name));
    Ok(())
}

/// What an existing Dockerfile, which `deploy` keeps, lacks for the VM path.
fn existing_dockerfile_warnings(dockerfile: &str, database: Option<&str>) -> Vec<&'static str> {
    let mut warnings = Vec::new();
    if !dockerfile.contains("Rullst.toml") {
        warnings.push(
            "The existing Dockerfile does not copy Rullst.toml, so the image lacks the trusted proxy. Add `COPY --chown=10001:10001 Rullst.toml /app/Rullst.toml` to its runtime stage.",
        );
    }
    if database == Some("Sqlite") && !dockerfile.contains("DATABASE_URL") {
        warnings.push(
            "The existing Dockerfile does not set DATABASE_URL, so the SQLite database would be opened in the read-only /app and the app would stop at startup. Add `ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc` to its runtime stage, or set DATABASE_URL in .env.production.",
        );
    }
    warnings
}

/// The commands that start the generated files on a VM.
fn next_steps(project_name: &str) -> String {
    format!(
        "\nOn the VM (Docker or Podman), from the project directory:\n  \
         1. Create .env.production with the production settings (APP_KEY, DATABASE_URL, ...).\n  \
         2. Point your domain's DNS at the VM and allow ports 80 and 443.\n  \
         3. DOMAIN=example.com docker compose up -d --build   (or: podman compose up -d --build)\n  \
         4. Migrate once per release: docker compose run --rm app /app/{project_name} db:migrate\n\
         Nix, Buildah, Podman Quadlet and Kubernetes: {ADVANCED_DEPLOYMENT}"
    )
}

/// How `Rullst.toml` came to trust the generated Caddy proxy.
#[derive(Debug, PartialEq, Eq)]
enum ProxyTrust {
    /// `trusted_proxies` was absent and now lists the proxy.
    Added,
    /// The proxy was already listed.
    AlreadyTrusted,
    /// An existing list without the proxy was left unchanged.
    OtherList,
}

/// Adds the generated Caddy proxy to `[security] trusted_proxies`, returning
/// the updated document when it changed. Without it every request reaches
/// the app from Caddy, so all clients share per-client limits such as the
/// starters' login and registration budget.
fn with_trusted_proxy(config: &str) -> Result<(ProxyTrust, Option<String>), String> {
    let mut document = config.parse::<toml_edit::DocumentMut>().map_err(|error| {
        // The parser's message quotes the offending line, which may hold a
        // secret such as a database URL.
        let line = error.span().map(|span| {
            config
                .get(..span.start)
                .unwrap_or(config)
                .matches('\n')
                .count()
                + 1
        });
        format!(
            "Rullst.toml is not valid TOML{}; fix it before running `cargo rullst deploy`",
            line.map_or_else(String::new, |line| format!(" (line {line})"))
        )
    })?;
    let security = document.entry("security").or_insert_with(toml_edit::table);
    // Comments cannot go inside an inline `security = { ... }` table.
    let standard_table = security.is_table();
    let security = security
        .as_table_like_mut()
        .ok_or("Rullst.toml `security` must be a table")?;
    if let Some(existing) = security.get("trusted_proxies") {
        let listed = existing.as_array().is_some_and(|networks| {
            networks
                .iter()
                .any(|network| network.as_str() == Some(VPS_PROXY_ADDRESS))
        });
        let trust = if listed {
            ProxyTrust::AlreadyTrusted
        } else {
            ProxyTrust::OtherList
        };
        return Ok((trust, None));
    }
    let networks = toml_edit::Array::from_iter([VPS_PROXY_ADDRESS]);
    security.insert("trusted_proxies", toml_edit::value(networks));
    if let Some(mut key) = security
        .key_mut("trusted_proxies")
        .filter(|_| standard_table)
    {
        key.leaf_decor_mut()
            .set_prefix("# Caddy in compose.yaml (`cargo rullst deploy`) reports the client.\n");
    }
    Ok((ProxyTrust::Added, Some(document.to_string())))
}

fn trust_proxy(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = match fs::read_to_string(path) {
        Ok(config) => config,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let (trust, updated) = with_trusted_proxy(&config).map_err(std::io::Error::other)?;
    if let Some(updated) = updated {
        fs::write(path, updated)?;
    }
    let message = match trust {
        ProxyTrust::Added => format!(
            "  ✅ Trusted the Caddy proxy ({VPS_PROXY_ADDRESS}) in Rullst.toml [security] trusted_proxies"
        )
        .green(),
        ProxyTrust::AlreadyTrusted => {
            format!("  ℹ️ Rullst.toml already trusts the Caddy proxy ({VPS_PROXY_ADDRESS}).").blue()
        }
        ProxyTrust::OtherList => format!(
            "  ⚠️ Rullst.toml [security] trusted_proxies does not list the Caddy proxy ({VPS_PROXY_ADDRESS}); add it, or every client shares the proxy's rate limits."
        )
        .yellow(),
    };
    println!("{message}");
    Ok(())
}

#[cfg(test)]
#[path = "deploy_tests.rs"]
mod tests;
