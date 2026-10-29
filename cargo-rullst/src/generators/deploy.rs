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
mod tests {
    use super::*;
    use rullst_core::config::RullstConfig;
    use rullst_core::security::TrustedProxyConfig;
    use std::net::IpAddr;

    fn trusts_caddy(config: &str) -> bool {
        let parsed = RullstConfig::from_toml(config).expect("valid Rullst.toml");
        let policy = TrustedProxyConfig::from_security_config(&parsed.security)
            .expect("valid trusted-proxy policy");
        let caddy: IpAddr = VPS_PROXY_ADDRESS.parse().expect("proxy address");
        policy.is_trusted(caddy) && !policy.is_trusted("172.31.250.11".parse().expect("peer"))
    }

    #[test]
    fn compose_pins_caddy_hardens_both_containers_and_checks_health() {
        // Behind Caddy every request reached the app from one dynamic bridge
        // address, so all clients shared the 10-per-minute login budget.
        assert!(COMPOSE_YAML.contains("ipv4_address: 172.31.250.10\n"));
        assert!(COMPOSE_YAML.contains("- subnet: 172.31.250.0/24\n"));
        assert_eq!(VPS_PROXY_ADDRESS, "172.31.250.10");
        assert_eq!(COMPOSE_YAML.matches("read_only: true\n").count(), 2);
        assert_eq!(COMPOSE_YAML.matches("cap_drop:\n      - ALL\n").count(), 2);
        assert_eq!(
            COMPOSE_YAML.matches("- no-new-privileges:true\n").count(),
            2
        );
        assert_eq!(COMPOSE_YAML.matches("cap_add:").count(), 1);
        assert!(COMPOSE_YAML.contains("cap_add:\n      - NET_BIND_SERVICE\n"));
        assert!(COMPOSE_YAML.contains("\"http://app:3000/health\"]"));
        assert!(COMPOSE_YAML.contains("env_file:\n      - .env.production\n"));
        assert!(COMPOSE_YAML.contains("- app_data:/app/data\n"));
        assert!(COMPOSE_YAML.contains("image: docker.io/library/caddy:2-alpine\n"));
        assert!(COMPOSE_YAML.contains("podman compose up -d --build"));
        assert!(!COMPOSE_YAML.contains("privileged: true"));
        assert!(
            !COMPOSE_YAML.contains('\t'),
            "YAML indentation is spaces only"
        );
        assert!(CADDYFILE.contains("\treverse_proxy app:3000 {\n"));
        assert!(CADDYFILE.contains("\t\thealth_uri /health\n"));
    }

    // Caddy's first active check runs before the app listens, and with a
    // 30-second interval the site answered 503 for up to 30 seconds after
    // every start (found by .github/check-vm-deploy.py).
    #[test]
    fn caddy_recovers_quickly_and_holds_requests_while_the_app_restarts() {
        let directives = CADDYFILE
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>();
        assert_eq!(
            directives,
            [
                "{$DOMAIN:localhost} {",
                "reverse_proxy app:3000 {",
                "health_uri /health",
                "health_interval 5s",
                "lb_try_duration 15s",
                "}",
                "}",
            ]
        );
        // `caddy fmt` indents with tabs; spaces log a warning at every start.
        assert!(
            CADDYFILE.lines().all(|line| !line.starts_with(' ')),
            "{CADDYFILE}"
        );
    }

    #[test]
    fn only_the_vm_path_remains_and_removed_targets_point_to_the_guide() {
        assert_eq!(check_platform(None), Ok(()));
        assert_eq!(check_platform(Some("VPS")), Ok(()));
        for removed in ["fly", "Fly.io", "railway", "render"] {
            let error = check_platform(Some(removed)).unwrap_err();
            assert!(error.contains("was removed in 13.0.0-alpha.4"), "{error}");
            assert!(error.ends_with("/advanced-deployment.html"), "{error}");
        }
        let unknown = check_platform(Some("flyio")).unwrap_err();
        assert!(unknown.starts_with("Unknown platform 'flyio'"));
    }

    #[test]
    fn deploy_writes_the_vm_files_once_and_never_through_a_link() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname = \"demo_app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        deploy_in(root.path(), None).unwrap();
        for file in ["Dockerfile", ".dockerignore", "compose.yaml", "Caddyfile"] {
            assert!(root.path().join(file).is_file(), "{file}");
        }
        assert_eq!(
            fs::read_to_string(root.path().join("compose.yaml")).unwrap(),
            COMPOSE_YAML
        );
        let dockerfile = fs::read_to_string(root.path().join("Dockerfile")).unwrap();
        assert!(
            dockerfile.contains("AS planner"),
            "the dependency-cache stage stays"
        );
        assert!(dockerfile.contains("COPY --chown=10001:10001 Rullst.toml /app/Rullst.toml"));
        assert!(trusts_caddy(
            &fs::read_to_string(root.path().join("Rullst.toml")).unwrap()
        ));

        fs::write(root.path().join("Caddyfile"), "custom\n").unwrap();
        deploy_in(root.path(), Some("vps")).unwrap();
        assert_eq!(
            fs::read_to_string(root.path().join("Caddyfile")).unwrap(),
            "custom\n"
        );

        #[cfg(unix)]
        {
            let linked = tempfile::tempdir().unwrap();
            fs::write(
                linked.path().join("Cargo.toml"),
                "[package]\nname = \"demo\"\n",
            )
            .unwrap();
            let target = linked.path().join("outside.txt");
            std::os::unix::fs::symlink(&target, linked.path().join("compose.yaml")).unwrap();
            deploy_in(linked.path(), None).unwrap();
            assert!(!target.exists(), "a dangling link is never followed");
        }
    }

    // A new project's `sqlite://db.sqlite` resolved inside the read-only /app
    // of the container, so the app stopped at startup (found by
    // .github/check-vm-deploy.py); the image keeps it in the data volume.
    #[test]
    fn deploy_keeps_a_sqlite_database_in_the_data_volume() {
        let in_volume = "ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc\n";
        for (config, expected) in [
            ("[database]\nurl = \"sqlite://db.sqlite\"\n", true),
            (
                "[database]\nurl = \"postgres://user:password@localhost:5432/db\"\n",
                false,
            ),
            ("", false),
        ] {
            let root = tempfile::tempdir().unwrap();
            fs::write(
                root.path().join("Cargo.toml"),
                "[package]\nname = \"demo_app\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
            fs::write(root.path().join("Rullst.toml"), config).unwrap();
            deploy_in(root.path(), None).unwrap();
            let dockerfile = fs::read_to_string(root.path().join("Dockerfile")).unwrap();
            assert_eq!(dockerfile.contains(in_volume), expected, "{config}");
            let deployed = fs::read_to_string(root.path().join("Rullst.toml")).unwrap();
            assert!(deployed.starts_with(config), "{deployed}");
        }
    }

    #[test]
    fn an_existing_dockerfile_is_kept_with_a_warning_for_what_it_lacks() {
        let copy = "COPY --chown=10001:10001 Rullst.toml /app/Rullst.toml\n";
        let current = format!("{copy}ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc\n");
        assert!(existing_dockerfile_warnings(&current, Some("Sqlite")).is_empty());
        let warnings = existing_dockerfile_warnings(copy, Some("Sqlite"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("ENV DATABASE_URL=sqlite:///app/data/rullst.db?mode=rwc"));
        assert!(existing_dockerfile_warnings(copy, None).is_empty());
        assert_eq!(
            existing_dockerfile_warnings("FROM scratch\n", None).len(),
            1
        );
        assert_eq!(
            existing_dockerfile_warnings("FROM scratch\n", Some("Sqlite")).len(),
            2
        );

        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\n",
        )
        .unwrap();
        fs::write(
            root.path().join("Rullst.toml"),
            "[database]\nurl = \"sqlite://db.sqlite\"\n",
        )
        .unwrap();
        fs::write(root.path().join("Dockerfile"), copy).unwrap();
        deploy_in(root.path(), None).unwrap();
        assert_eq!(
            fs::read_to_string(root.path().join("Dockerfile")).unwrap(),
            copy
        );
    }

    // TM-DEPLOY-05: validated generator inputs cannot cross into Dockerfile syntax.
    #[test]
    fn deploy_refuses_a_package_name_that_could_inject_dockerfile_syntax() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname = \"demo\\\"]\\nRUN touch /compromised\"\n",
        )
        .unwrap();
        let error = deploy_in(root.path(), None).unwrap_err();
        assert!(error.to_string().contains("not a valid Cargo package name"));
        for file in ["Dockerfile", "compose.yaml", "Caddyfile", "Rullst.toml"] {
            assert!(!root.path().join(file).exists(), "{file}");
        }
        assert!(safe_package_name("my-app_2"));
        let long = "a".repeat(65);
        for unsafe_name in ["", "a b", "a;b", "a\"b", "a/b", "ä", long.as_str()] {
            assert!(!safe_package_name(unsafe_name), "{unsafe_name}");
        }
    }

    #[test]
    fn next_steps_cover_docker_podman_and_the_migration() {
        let text = next_steps("demo_app");
        assert!(text.contains("DOMAIN=example.com docker compose up -d --build"));
        assert!(text.contains("podman compose up -d --build"));
        assert!(text.contains("docker compose run --rm app /app/demo_app db:migrate"));
        assert!(text.ends_with("/advanced-deployment.html"));
    }

    #[test]
    fn deploy_trusts_only_the_generated_proxy() {
        let (trust, created) = with_trusted_proxy("").expect("empty configuration");
        assert_eq!(trust, ProxyTrust::Added);
        let created = created.expect("new configuration");
        assert!(created.starts_with("[security]\n# Caddy in compose.yaml"));
        assert!(trusts_caddy(&created));

        let saas = "[database]\nurl = \"sqlite://db.sqlite\"\n\n[security]\ncsrf_signed_webhook_paths = [\"/billing/webhook\"]\n";
        let (trust, updated) = with_trusted_proxy(saas).expect("SaaS configuration");
        assert_eq!(trust, ProxyTrust::Added);
        let updated = updated.expect("updated configuration");
        assert!(updated.starts_with(saas));
        assert!(trusts_caddy(&updated));
        let parsed = RullstConfig::from_toml(&updated).expect("valid Rullst.toml");
        assert_eq!(
            parsed.security.csrf_signed_webhook_paths,
            ["/billing/webhook"]
        );
        assert_eq!(
            with_trusted_proxy(&updated).expect("second run"),
            (ProxyTrust::AlreadyTrusted, None)
        );

        let inline = "security = { coep = \"require-corp\" }\n";
        let (trust, updated) = with_trusted_proxy(inline).expect("inline table");
        assert_eq!(trust, ProxyTrust::Added);
        assert!(trusts_caddy(&updated.expect("updated inline table")));

        // An operator's own list is never rewritten.
        let custom = "[security]\ntrusted_proxies = [\"10.0.0.0/8\"]\n";
        assert_eq!(
            with_trusted_proxy(custom).expect("custom list"),
            (ProxyTrust::OtherList, None)
        );
    }

    #[test]
    fn unparsable_configuration_is_reported_without_its_contents() {
        let error = with_trusted_proxy("[database]\nurl = \"postgres://user:hunter2@db\nbroken")
            .expect_err("invalid TOML");
        assert!(error.contains("Rullst.toml is not valid TOML (line "));
        assert!(!error.contains("hunter2"));
    }
}
