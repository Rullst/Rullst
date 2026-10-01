#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

#[test]
fn advisory_exception_ids_are_strictly_validated() {
    assert!(validate_audit_ignores(&["RUSTSEC-2099-0001".to_string()]).is_ok());
    for invalid in [
        "rustsec-2099-0001",
        "RUSTSEC-99-0001",
        "RUSTSEC-2099-001",
        "RUSTSEC-2099-0001 --quiet",
    ] {
        assert!(validate_audit_ignores(&[invalid.to_string()]).is_err());
    }
}

#[test]
fn advisory_exceptions_are_forwarded_as_distinct_cargo_audit_arguments() {
    assert_eq!(
        cargo_audit_arguments(&[
            "RUSTSEC-2099-0001".to_string(),
            "RUSTSEC-2099-0002".to_string(),
        ]),
        [
            "audit",
            "--ignore",
            "RUSTSEC-2099-0001",
            "--ignore",
            "RUSTSEC-2099-0002",
        ]
    );
}

#[cfg(unix)]
#[test]
fn source_scans_do_not_follow_a_symlinked_directory_loop() {
    let project = tempfile::tempdir().expect("temporary project");
    let src = project.path().join("src");
    fs::create_dir_all(&src).expect("source directory");
    fs::write(
        src.join("lib.rs"),
        "pub unsafe fn unchecked() {}\nfn routes() { get(\"/users/:id\" => show); }\n",
    )
    .expect("source fixture");
    // The old walk followed this link until the kernel's symlink bound and
    // reported one copy of every finding per level.
    std::os::unix::fs::symlink(".", src.join("loop")).expect("loop link");

    let (unsafe_count, _) = scan_unsafe_code(&src);
    assert_eq!(unsafe_count, 1);
    let (idor_count, warnings) = scan_idor_vulnerabilities(&src);
    assert_eq!(idor_count, 1, "{warnings:?}");
}
