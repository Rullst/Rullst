use super::*;

fn executable_file(path: &Path) {
    std::fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn binary_name() -> String {
    format!("cargo-rullst-extras{}", std::env::consts::EXE_SUFFIX)
}

#[test]
fn the_install_hint_pins_this_exact_version() {
    assert_eq!(
        install_command(),
        format!(
            "cargo install cargo-rullst-extras --version ={} --locked",
            env!("CARGO_PKG_VERSION")
        )
    );
    let report = missing_report("green");
    assert_eq!(
        report.title,
        "`cargo rullst green` needs cargo-rullst-extras"
    );
    assert!(
        report
            .happened
            .contains("no `cargo-rullst-extras` executable was found on PATH")
    );
    let fix = report.fix.unwrap_or_default();
    assert!(fix.ends_with(&install_command()), "{fix}");
    assert!(
        report
            .docs
            .is_some_and(|docs| docs.ends_with("#extras-cargo-rullst-extras"))
    );
}

#[test]
fn only_ai_and_green_are_forwarded() {
    assert!(is_extras("ai"));
    assert!(is_extras("green"));
    for name in ["footprint", "doctor", "Ai", ""] {
        assert!(!is_extras(name), "{name}");
    }
    let names: Vec<String> = commands()
        .map(|command| command.get_name().to_string())
        .collect();
    assert_eq!(names, ["ai", "green"]);
    for command in commands() {
        let about = command.get_about().map(ToString::to_string);
        assert!(about.is_some_and(|about| about.starts_with("(extras) ")));
    }
}

#[test]
fn stub_arguments_reach_the_extras_unparsed() {
    let matches = crate::command()
        .try_get_matches_from(["rullst", "green", "--help", "--json", "extra"])
        .unwrap();
    let (name, green) = matches.subcommand().unwrap();
    assert_eq!(name, "green");
    let arguments: Vec<&String> = green.get_many::<String>("arguments").unwrap().collect();
    assert_eq!(arguments, ["--help", "--json", "extra"]);
    let bare = crate::command()
        .try_get_matches_from(["rullst", "ai"])
        .unwrap();
    let (_, ai) = bare.subcommand().unwrap();
    assert!(ai.get_many::<String>("arguments").is_none());
}

#[test]
fn the_executable_is_found_only_in_absolute_path_entries() {
    let root = tempfile::tempdir().unwrap();
    let empty = root.path().join("empty");
    let installed = root.path().join("bin");
    std::fs::create_dir_all(&empty).unwrap();
    std::fs::create_dir_all(&installed).unwrap();
    let expected = installed.join(binary_name());
    executable_file(&expected);

    let path = std::env::join_paths([empty.clone(), installed.clone()]).unwrap();
    assert_eq!(find(Some(&path)), Some(expected.clone()));
    assert_eq!(find(None), None);
    let without = std::env::join_paths([empty.clone()]).unwrap();
    assert_eq!(find(Some(&without)), None);

    // A relative entry (here the current directory's `bin`) is never used.
    let relative = std::env::join_paths([PathBuf::from("bin"), PathBuf::from("")]).unwrap();
    assert_eq!(find(Some(&relative)), None);

    // A directory with the executable's name is not an executable.
    let decoy = root.path().join("decoy");
    std::fs::create_dir_all(decoy.join(binary_name())).unwrap();
    let path = std::env::join_paths([decoy, installed]).unwrap();
    assert_eq!(find(Some(&path)), Some(expected));
}

#[cfg(unix)]
#[test]
fn a_file_without_execute_permission_is_skipped() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let plain = root.path().join("plain");
    std::fs::create_dir_all(&plain).unwrap();
    let candidate = plain.join(binary_name());
    std::fs::write(&candidate, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&candidate, std::fs::Permissions::from_mode(0o644)).unwrap();
    let path = std::env::join_paths([plain]).unwrap();
    assert_eq!(find(Some(&path)), None);
}

#[test]
fn a_missing_executable_fails_with_status_1() {
    let root = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([root.path()]).unwrap();
    let error = run_with(Some(&path), "ai", &["status".to_string()]).unwrap_err();
    assert_eq!(error.to_string(), "cargo-rullst-extras is not installed");
    let error: &(dyn Error + 'static) = error.as_ref();
    assert_eq!(error_report::report(error, &[]), 1);
}

#[cfg(unix)]
#[test]
fn exit_codes_and_signals_pass_through() {
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(exit_code(ExitStatus::from_raw(7 << 8)), 7);
    assert_eq!(exit_code(ExitStatus::from_raw(255 << 8)), 255);
    // Killed by SIGKILL (9): 128 + 9, as shells report it.
    assert_eq!(exit_code(ExitStatus::from_raw(9)), 137);
    assert!(passthrough(ExitStatus::from_raw(0), "ai").is_ok());
    let error = passthrough(ExitStatus::from_raw(3 << 8), "green").unwrap_err();
    assert!(error.to_string().starts_with("`cargo rullst green` failed"));
    let error: &(dyn Error + 'static) = error.as_ref();
    assert_eq!(error_report::report(error, &[]), 3);
}
