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
