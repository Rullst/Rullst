//! Proxy trust written to Rullst.toml and provider CLI outcomes.
use super::*;

#[test]
fn a_parse_error_names_its_line() {
    let error = with_trusted_vps_proxy("[app]\nname = \"shop\"\nbroken = \n").unwrap_err();
    assert_eq!(
        error,
        "Rullst.toml is not valid TOML (line 3); fix it before running `deploy --platform vps`"
    );
}

#[test]
fn trusting_the_vps_proxy_creates_or_keeps_the_configuration() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("Rullst.toml");
    trust_vps_proxy(&config).unwrap();
    let written = fs::read_to_string(&config).unwrap();
    assert!(
        written.contains(&format!("trusted_proxies = [\"{VPS_PROXY_ADDRESS}\"]")),
        "{written}"
    );
    trust_vps_proxy(&config).unwrap();
    assert_eq!(fs::read_to_string(&config).unwrap(), written);
}

#[cfg(unix)]
#[test]
fn provider_commands_report_success_failure_and_absence() {
    use std::os::unix::fs::PermissionsExt;
    assert!(run_provider_cli("sh", &["-c", "exit 0"]).unwrap());
    let failed = run_provider_cli("sh", &["-c", "exit 3"]).unwrap_err();
    assert!(
        failed
            .to_string()
            .starts_with("`sh -c exit 3` failed (exit status: 3)"),
        "{failed}"
    );
    assert!(!run_provider_cli("rullst-missing-provider-cli", &[]).unwrap());
    let root = tempfile::tempdir().unwrap();
    let not_executable = root.path().join("provider");
    fs::write(&not_executable, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&not_executable, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(run_provider_cli(not_executable.to_str().unwrap(), &[]).is_err());
}

/// Runs `deploy --platform <RULLST_DEPLOY_PLATFORM>` when a parent test names
/// a report file; otherwise does nothing.
#[test]
fn deploy_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_DEPLOY_REPORT") else {
        return;
    };
    let platform = std::env::var("RULLST_DEPLOY_PLATFORM").unwrap();
    let outcome = match run_deploy(Some(&platform)) {
        Ok(()) => "ok".to_string(),
        Err(error) => error.to_string(),
    };
    fs::write(report, outcome).unwrap();
}

/// The outcome and output of `deploy --platform <platform>` in `project`,
/// with no provider CLI on `PATH`.
fn deploy(project: &Path, platform: &str) -> (String, String) {
    let report = project.join("report");
    let empty = tempfile::tempdir().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "generators::deploy::provider_tests::deploy_child_fixture",
            "--nocapture",
        ])
        .current_dir(project)
        .env("PATH", empty.path())
        .env("NO_COLOR", "1")
        .env("RULLST_DEPLOY_REPORT", &report)
        .env("RULLST_DEPLOY_PLATFORM", platform)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    let outcome = fs::read_to_string(&report).unwrap();
    fs::remove_file(report).unwrap();
    (outcome, String::from_utf8(output.stdout).unwrap())
}

fn project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Cargo.toml"),
        "[package]\nname = \"My_Shop\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    project
}

#[test]
fn fly_and_railway_get_their_files_and_manual_steps() {
    let project = project();
    let root = project.path();
    let (outcome, stdout) = deploy(root, "fly");
    assert_eq!(outcome, "ok");
    assert!(
        fs::read_to_string(root.join("fly.toml"))
            .unwrap()
            .contains("app = \"my-shop\"")
    );
    assert!(root.join("Dockerfile").is_file());
    assert!(
        !root.join("Rullst.toml").exists(),
        "only a VPS trusts Caddy"
    );
    assert!(stdout.contains("Fly CLI ('flyctl') not found"), "{stdout}");

    fs::write(root.join("fly.toml"), "app = \"kept\"\n").unwrap();
    let (_, stdout) = deploy(root, "fly");
    assert!(stdout.contains("Existing fly.toml retained."), "{stdout}");
    assert_eq!(
        fs::read_to_string(root.join("fly.toml")).unwrap(),
        "app = \"kept\"\n"
    );

    let (outcome, stdout) = deploy(root, "railway");
    assert_eq!(outcome, "ok");
    assert!(
        fs::read_to_string(root.join("railway.json"))
            .unwrap()
            .contains("My_Shop")
    );
    assert!(
        stdout.contains("Railway CLI ('railway') not found"),
        "{stdout}"
    );

    let (outcome, stdout) = deploy(root, "render");
    assert_eq!(outcome, "ok");
    assert!(
        fs::read_to_string(root.join("render.yaml"))
            .unwrap()
            .contains("My_Shop")
    );
    assert!(stdout.contains("Render Blueprint generated"), "{stdout}");
}

#[test]
fn a_vps_gets_compose_caddy_and_a_trusted_proxy() {
    let project = project();
    let root = project.path();
    let (outcome, stdout) = deploy(root, "vps");
    assert_eq!(outcome, "ok");
    for file in ["docker-compose.prod.yml", "Caddyfile", "Dockerfile"] {
        assert!(root.join(file).is_file(), "{file}");
    }
    assert!(
        fs::read_to_string(root.join("Rullst.toml"))
            .unwrap()
            .contains(VPS_PROXY_ADDRESS)
    );
    assert!(!stdout.contains("⚠️"), "{stdout}");

    // A compose file without the pinned address and a Dockerfile that does
    // not copy Rullst.toml are reported, not replaced.
    fs::write(root.join("docker-compose.prod.yml"), "services: {}\n").unwrap();
    fs::write(root.join("Dockerfile"), "FROM scratch\n").unwrap();
    fs::write(root.join("Caddyfile"), "kept\n").unwrap();
    let (outcome, stdout) = deploy(root, "vps");
    assert_eq!(outcome, "ok");
    assert!(stdout.contains("does not pin Caddy"), "{stdout}");
    assert!(stdout.contains("does not copy Rullst.toml"), "{stdout}");
    assert_eq!(
        fs::read_to_string(root.join("Caddyfile")).unwrap(),
        "kept\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("docker-compose.prod.yml")).unwrap(),
        "services: {}\n"
    );
}

#[cfg(unix)]
#[test]
fn an_unreadable_configuration_is_never_overwritten() {
    use std::os::unix::fs::PermissionsExt;
    if rustix::process::geteuid().is_root() {
        return; // Permissions do not stop root from reading.
    }
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("Rullst.toml");
    fs::write(&config, "[app]\nname = \"shop\"\n").unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o200)).unwrap();
    let outcome = trust_vps_proxy(&config);
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(outcome.is_err());
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        "[app]\nname = \"shop\"\n"
    );
}
