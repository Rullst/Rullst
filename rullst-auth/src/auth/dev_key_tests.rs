//! State-directory selection, project key names and test-harness detection.
use super::*;
use std::collections::HashMap;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, OsString> = pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
        .collect();
    move |name| map.get(name).cloned()
}

#[cfg(not(windows))]
#[test]
fn unix_state_directory_follows_xdg_then_home() {
    assert_eq!(
        state_directory(env(&[("XDG_STATE_HOME", "/state"), ("HOME", "/home/ana")])),
        Some(PathBuf::from("/state/rullst/dev-keys"))
    );
    assert_eq!(
        state_directory(env(&[("HOME", "/home/ana")])),
        Some(PathBuf::from("/home/ana/.local/state/rullst/dev-keys"))
    );
    // Relative or empty values are ignored, as the XDG specification says.
    assert_eq!(
        state_directory(env(&[
            ("XDG_STATE_HOME", "relative"),
            ("HOME", "/home/ana")
        ])),
        Some(PathBuf::from("/home/ana/.local/state/rullst/dev-keys"))
    );
    assert_eq!(
        state_directory(env(&[("XDG_STATE_HOME", ""), ("HOME", "")])),
        None
    );
    assert_eq!(state_directory(env(&[("LOCALAPPDATA", "/local")])), None);
}

#[cfg(windows)]
#[test]
fn windows_state_directory_is_local_app_data() {
    assert_eq!(
        state_directory(env(&[("LOCALAPPDATA", r"C:\Users\ana\AppData\Local")])),
        Some(PathBuf::from(r"C:\Users\ana\AppData\Local\rullst\dev-keys"))
    );
    assert_eq!(
        state_directory(env(&[
            ("XDG_STATE_HOME", r"C:\state"),
            ("HOME", r"C:\Users\ana")
        ])),
        None
    );
    assert_eq!(state_directory(env(&[("LOCALAPPDATA", "relative")])), None);
}

#[test]
fn each_project_has_its_own_key_file() {
    let directory = Path::new("keys");
    let first = key_file(directory, Path::new("/work/app"));
    assert_eq!(first, key_file(directory, Path::new("/work/app")));
    assert_ne!(first, key_file(directory, Path::new("/work/other")));
    assert_eq!(first.parent(), Some(directory));
    let name = first.file_name().unwrap().to_str().unwrap();
    // SHA-256("/work/app"), first 16 bytes.
    assert_eq!(name, "70467eff2e0a236497f0693901c782c6.key");
}

#[test]
fn only_binaries_in_a_cargo_deps_directory_are_test_harnesses() {
    for (exe, harness) in [
        ("/app/target/debug/deps/app-1a2b3c", true),
        (
            "/app/target/x86_64-unknown-linux-gnu/release/deps/api_test-9f",
            true,
        ),
        ("/app/target/debug/app", false),
        ("/app/target/debug/examples/demo", false),
        ("/usr/local/bin/app", false),
        ("/opt/deps-tools/app", false),
        ("app", false),
    ] {
        assert_eq!(is_test_harness(Some(Path::new(exe))), harness, "{exe}");
    }
    assert!(!is_test_harness(None));
}
