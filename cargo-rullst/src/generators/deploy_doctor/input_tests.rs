//! Bounded file reads, the literal dotenv profile and input selection.
use super::*;

fn ok<T>(result: Result<T, InputError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{}", error.code()),
    }
}

fn file_error<T: std::fmt::Debug>(result: Result<T, InputError>) -> &'static str {
    match result {
        Ok(value) => panic!("accepted: {value:?}"),
        Err(error) => error.code(),
    }
}

#[test]
fn input_errors_have_exact_codes_and_guidance() {
    for (error, code, guidance) in [
        (
            InputError::File,
            "input_file",
            "Use a readable regular UTF-8 file at most 64 KiB",
        ),
        (
            InputError::Toml,
            "input_toml",
            "Correct the selected TOML syntax",
        ),
        (
            InputError::Env,
            "input_env_file",
            "Use at most 512 unique KEY=value lines",
        ),
        (
            InputError::Process,
            "input_process_env",
            "The three selected process variables",
        ),
    ] {
        assert_eq!(error.code(), code);
        assert!(error.guidance().starts_with(guidance), "{code}");
    }
}

#[test]
fn reads_return_bounded_regular_utf8_files() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let path = root.join("Rullst.toml");
    fs::write(&path, "[app]\n").unwrap();
    assert_eq!(ok(read(&path, false)).as_deref(), Some("[app]\n"));
    fs::write(&path, vec![b'#'; 64 * 1024]).unwrap();
    assert_eq!(ok(read(&path, false)).unwrap().len(), 65_536);
    fs::write(&path, vec![b'#'; 64 * 1024 + 1]).unwrap();
    assert_eq!(file_error(read(&path, false)), "input_file");
    fs::write(&path, [0xff, 0xfe]).unwrap();
    assert_eq!(file_error(read(&path, false)), "input_file");
    let missing = root.join("missing.toml");
    assert_eq!(ok(read(&missing, true)), None);
    assert_eq!(file_error(read(&missing, false)), "input_file");
    assert_eq!(file_error(read(&root, true)), "input_file");
}

#[cfg(unix)]
#[test]
fn linked_files_and_ancestors_are_refused() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    fs::create_dir(root.join("real")).unwrap();
    fs::write(root.join("real/Rullst.toml"), "[app]\n").unwrap();
    symlink(root.join("real"), root.join("alias")).unwrap();
    symlink(root.join("real/Rullst.toml"), root.join("link.toml")).unwrap();
    assert_eq!(
        file_error(read(&root.join("alias/Rullst.toml"), false)),
        "input_file"
    );
    assert_eq!(
        file_error(read(&root.join("link.toml"), false)),
        "input_file"
    );
}

/// sysfs reports a 4096-byte size for every attribute and returns less.
#[cfg(target_os = "linux")]
#[test]
fn a_file_shorter_than_its_reported_size_is_refused() {
    let path = Path::new("/sys/devices/system/cpu/online");
    if path.exists() {
        assert_eq!(file_error(read(path, false)), "input_file");
    }
}

#[test]
fn the_literal_dotenv_profile_is_exact() {
    let parsed = env_file(
        "# comment\n\nRULLST_ENV=production\nAPP_KEY='quoted value'\t# note\nexport APP_ENV=\"staging\"\nOTHER1=ignored\n",
    )
    .unwrap_or_else(|_| panic!("valid profile"));
    assert_eq!(
        parsed,
        BTreeMap::from([
            ("APP_ENV".to_string(), "staging".to_string()),
            ("APP_KEY".to_string(), "quoted value".to_string()),
            ("RULLST_ENV".to_string(), "production".to_string()),
        ])
    );
    assert!(env_file(&format!("{}=x\n", "K".repeat(128))).is_ok());
    for invalid in [
        format!("{}=x\n", "K".repeat(129)),
        "=x\n".to_string(),
        "1KEY=x\n".to_string(),
        "KEY-NAME=x\n".to_string(),
        "APP_KEY='x'#tail\n".to_string(),
        "APP_KEY=x\"y\n".to_string(),
        "APP_KEY='\u{7}'\n".to_string(),
    ] {
        assert!(env_file(&invalid).is_err(), "{invalid:?}");
    }
}

#[test]
fn explicit_inputs_select_the_environment_and_target() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    fs::write(root.join("Rullst.toml"), "[app]\nenv = 'development'\n").unwrap();
    fs::write(root.join("selected.env"), "RULLST_ENV=staging\n").unwrap();
    let load_with = |target: &str| {
        let matches = super::super::command()
            .try_get_matches_from([
                "deploy:doctor",
                "--config",
                root.join("Rullst.toml").to_str().unwrap(),
                "--env-file",
                root.join("selected.env").to_str().unwrap(),
                "--target",
                target,
            ])
            .unwrap();
        match load(&matches) {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("{}", error.code()),
        }
    };
    let staging = load_with("staging");
    assert!(staging.environment_selected && staging.staging && staging.config_present);
    assert_eq!(
        staging.environment.get("RULLST_ENV").map(String::as_str),
        Some("staging")
    );
    assert!(!load_with("production").staging);
}

/// Loads `--process-env` in a child test process with its own environment.
#[test]
fn process_environment_child_fixture() {
    let Some(report) = std::env::var_os("RULLST_DEPLOY_DOCTOR_REPORT") else {
        return;
    };
    let config = std::env::var("RULLST_DEPLOY_DOCTOR_CONFIG").unwrap();
    let matches = super::super::command()
        .try_get_matches_from(["deploy:doctor", "--config", &config, "--process-env"])
        .unwrap();
    let text = match load(&matches) {
        Ok(snapshot) => snapshot
            .environment
            .iter()
            .map(|(key, value)| format!("{key}:{}", value.len()))
            .collect::<Vec<_>>()
            .join(","),
        Err(error) => error.code().to_string(),
    };
    fs::write(report, text).unwrap();
}

fn process_environment(key: &str) -> String {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    fs::write(root.join("Rullst.toml"), "").unwrap();
    let report = root.join("report");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "generators::deploy_doctor::input::parsing_tests::process_environment_child_fixture",
            "--nocapture",
        ])
        .env("RULLST_DEPLOY_DOCTOR_REPORT", &report)
        .env("RULLST_DEPLOY_DOCTOR_CONFIG", root.join("Rullst.toml"))
        .env_remove("RULLST_ENV")
        .env_remove("APP_ENV")
        .env("APP_KEY", key)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    fs::read_to_string(report).unwrap()
}

#[test]
fn process_values_are_bounded_and_absent_ones_are_skipped() {
    assert_eq!(process_environment(&"k".repeat(8192)), "APP_KEY:8192");
    assert_eq!(process_environment(&"k".repeat(8193)), "input_process_env");
}
