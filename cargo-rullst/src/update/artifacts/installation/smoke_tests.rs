//! The candidate version probe accepts only one exact, quiet version line.
use super::*;

#[cfg(unix)]
fn probe(script: &str) -> Result<(), ArtifactError> {
    use std::os::unix::fs::PermissionsExt;
    let manifest = super::super::super::tests::fixture_manifest();
    let directory = tempfile::tempdir().unwrap();
    for name in state::executable_names(&manifest.target) {
        let path = directory.path().join(name);
        fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    smoke::verify(directory.path(), &manifest)
}

/// Retries a probe that must pass: another test thread forking while a script
/// was open for writing makes executing it fail briefly with ETXTBSY.
#[cfg(unix)]
fn passing(script: &str) {
    let mut result = probe(script);
    for _ in 0..20 {
        if result.is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        result = probe(script);
    }
    result.unwrap();
}

#[cfg(unix)]
#[test]
fn only_one_exact_quiet_version_line_passes() {
    passing("echo 'rullst 12.1.0'");
    passing("printf '%-4096s' 'cargo-rullst 12.1.0'");
    for rejected in [
        "printf '%-4097s' 'rullst 12.1.0'",
        "echo 'rullst 12.1.0'; echo 'warning' >&2",
        "echo 'rullst 12.1.0 extra'",
        "echo 'other 12.1.0'",
        "echo 'rullst 12.2.0'",
        "echo 'rullst 12.1.0'; exit 1",
    ] {
        assert!(probe(rejected).is_err(), "{rejected}");
    }
}
