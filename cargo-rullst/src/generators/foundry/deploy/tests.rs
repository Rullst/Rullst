use super::*;

fn test_config(auto_https: &str) -> FoundryConfig {
    FoundryConfig {
        app_name: "demo".to_string(),
        domain: "demo.example".to_string(),
        port: "3000".to_string(),
        host: "192.0.2.10".to_string(),
        user: "root".to_string(),
        ssh_key: String::new(),
        ssh_port: "22".to_string(),
        provider: "vps".to_string(),
        db_type: "sqlite".to_string(),
        profile: "release".to_string(),
        target_triple: String::new(),
        auto_https: auto_https.to_string(),
        env_vars: vec![("RULLST_ENV".to_string(), "production".to_string())],
    }
}

#[test]
fn configuration_requires_a_preinstalled_caddy_and_blocks_reload_failure() {
    let command = render_configure_command(&test_config("false"), "demo");
    assert!(command.contains(":80 {"));
    assert!(command.contains("exit 1"));
    assert!(command.contains("caddy validate --config"));
    assert!(command.contains("/tmp/rullst_demo.upload"));
    assert!(command.contains("mv -f /opt/rullst/demo/bin/demo.next /opt/rullst/demo/bin/demo"));
    assert!(command.contains("chmod 600 /opt/rullst/demo/config/.env.next"));
    assert!(command.contains("Caddyfile.previous"));
    assert!(command.contains("systemctl reload caddy || systemctl restart caddy"));
    assert!(!command.contains("caddyserver.com/install.sh"));
    assert!(!command.contains("docker rm"));
    assert!(!command.contains("pkill"));
    assert!(!command.contains("systemctl restart caddy 2>/dev/null || true"));
}

#[test]
fn the_application_listens_on_the_port_caddy_and_the_probe_use() {
    // The old environment file had no PORT, so the app kept its fallback port
    // while Caddy proxied to app.port.
    let mut cfg = test_config("true");
    cfg.port = "8080".to_string();
    let command = render_configure_command(&cfg, "demo");
    assert!(command.contains("reverse_proxy localhost:8080"));
    assert!(command.contains("\nPORT=\"8080\"\n"));

    cfg.port = String::new();
    cfg.env_vars.push(("PORT".to_string(), "8081".to_string()));
    let command = render_configure_command(&cfg, "demo");
    assert!(command.contains("reverse_proxy localhost:8081"));
    assert_eq!(command.matches("PORT=").count(), 1);

    cfg.env_vars.retain(|(name, _)| name != "PORT");
    let command = render_configure_command(&cfg, "demo");
    assert!(command.contains("PORT=\"3000\""));
}

#[test]
fn provisioning_requires_reviewed_tools_and_uses_an_app_specific_root() {
    let command = render_provision_command(&test_config("false"));
    assert!(command.contains("command -v curl"));
    assert!(command.contains("command -v systemctl"));
    assert!(command.contains("command -v caddy"));
    assert!(command.contains("/opt/rullst/demo/data"));
    assert!(command.contains("/opt/rullst/demo/config"));
    assert!(!command.contains("apt-get"));
    assert!(!command.contains("yum"));
    assert!(!command.contains("curl |"));
}

#[test]
fn systemd_environment_values_are_quoted_and_escaped() {
    assert_eq!(
        escape_systemd_env_value("space and \\\"quote"),
        "space and \\\\\\\"quote"
    );
    let mut cfg = test_config("false");
    cfg.env_vars = vec![("EXAMPLE".to_string(), "space and \\\"quote".to_string())];
    let command = render_configure_command(&cfg, "demo");
    assert!(command.contains(r#"EXAMPLE="space and \\\"quote""#));
}

#[test]
fn the_service_runs_as_a_dedicated_unprivileged_sandboxed_account() {
    let provision = render_provision_command(&test_config("false"));
    assert!(provision.contains("useradd --system --user-group --no-create-home"));
    assert!(
        provision
            .contains("install -d -m 0750 -o rullst-demo -g rullst-demo /opt/rullst/demo/data")
    );
    assert!(provision.contains("chown -R -h rullst-demo:rullst-demo /opt/rullst/demo/data"));
    assert!(provision.contains("install -d -m 0700 /opt/rullst/demo/config"));

    let command = render_configure_command(&test_config("false"), "demo");
    for directive in [
        "User=rullst-demo",
        "Group=rullst-demo",
        "NoNewPrivileges=yes",
        "CapabilityBoundingSet=\n",
        "ProtectSystem=strict",
        "ReadWritePaths=/opt/rullst/demo/data",
        "PrivateTmp=yes",
    ] {
        assert!(command.contains(directive), "missing {directive}");
    }
    assert!(!command.contains("AmbientCapabilities"));

    let mut privileged = test_config("false");
    privileged.port = "80".to_string();
    let privileged = render_configure_command(&privileged, "demo");
    assert!(privileged.contains("AmbientCapabilities=CAP_NET_BIND_SERVICE"));

    #[cfg(unix)]
    for script in [provision, command, privileged] {
        let mut shell = std::process::Command::new("sh")
            .arg("-n")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        shell
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        assert!(shell.wait().unwrap().success(), "invalid shell:\n{script}");
    }
}
