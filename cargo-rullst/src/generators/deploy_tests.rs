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
