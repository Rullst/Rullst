//! The VM deployment files, the trusted proxy and what `deploy` refuses.
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
    let error = with_trusted_proxy("[database]\nurl = \"sqlite://db.sqlite\"\n= broken\n")
        .expect_err("invalid TOML");
    assert_eq!(
        error,
        "Rullst.toml is not valid TOML (line 3); fix it before running `cargo rullst deploy`"
    );
}

// Treating an unreadable Rullst.toml like a missing one would replace it
// with a file that holds only the proxy setting.
#[test]
fn an_unreadable_configuration_stops_deploy_before_any_write() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\n",
    )
    .unwrap();
    let config = b"[database]\nurl = \"\xff\"\n";
    fs::write(root.path().join("Rullst.toml"), config).unwrap();
    let error = deploy_in(root.path(), None).unwrap_err();
    assert_eq!(error.to_string(), "stream did not contain valid UTF-8");
    assert_eq!(fs::read(root.path().join("Rullst.toml")).unwrap(), config);
    for file in ["Dockerfile", "compose.yaml", "Caddyfile"] {
        assert!(!root.path().join(file).exists(), "{file}");
    }
}

/// Runs `deploy` in the current directory when a parent test names a report
/// file; otherwise does nothing.
#[test]
fn deploy_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_DEPLOY_REPORT") else {
        return;
    };
    let outcome = match run_deploy(Some("vps")) {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    fs::write(report, outcome).unwrap();
}

#[test]
fn the_deploy_command_writes_into_the_current_directory() {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname = \"demo_app\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let report = project.path().join("report");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "generators::deploy::tests::deploy_child_fixture"])
        .current_dir(project.path())
        .env("RULLST_DEPLOY_REPORT", &report)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(fs::read_to_string(report).unwrap(), "ok");
    assert_eq!(
        fs::read_to_string(project.path().join("compose.yaml")).unwrap(),
        COMPOSE_YAML
    );
    assert_eq!(
        fs::read_to_string(project.path().join("Caddyfile")).unwrap(),
        CADDYFILE
    );
    assert!(trusts_caddy(
        &fs::read_to_string(project.path().join("Rullst.toml")).unwrap()
    ));
}
