use base64::{Engine as _, engine::general_purpose};
use rullst_auth::{AuthError, get_app_key, make_login_cookie, validate_app_key};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const CHILD_CASE: &str = "RULLST_APP_KEY_TEST_CASE";
const VALID_KEY: &str = "0123456789abcdefghijklmnopqrstuv";

struct IsolatedDirectory(PathBuf);

impl IsolatedDirectory {
    fn new(case: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rullst-auth-app-key-{}-{case}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("isolated test directory should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for IsolatedDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_isolated_case(case: &str, setup: impl FnOnce(&Path)) {
    let directory = IsolatedDirectory::new(case);
    setup(directory.path());

    let mut command = Command::new(std::env::current_exe().expect("test executable should exist"));
    command
        .arg("--exact")
        .arg("app_key_resolution_child")
        .arg("--nocapture")
        .env(CHILD_CASE, case)
        .env_remove("APP_KEY")
        .env_remove("APP_ENV")
        .env_remove("RULLST_ENV")
        .current_dir(directory.path());

    match case {
        "process_environment_precedes_dotenv" | "secure_cookie_in_production" => {
            command.env("APP_KEY", VALID_KEY);
        }
        _ => {}
    }
    if case == "secure_cookie_in_production" {
        command.env("RULLST_ENV", "production");
    }
    #[cfg(unix)]
    if case == "non_unicode_environment_is_rejected" {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        command.env("APP_ENV", OsString::from_vec(vec![0xff]));
    }

    let output = command
        .output()
        .expect("isolated app-key test should start");

    assert!(
        output.status.success(),
        "case {case} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// TM-AUTH-02: secure environments reject missing/invalid application keys.
fn app_key_resolution_is_fail_closed_and_durable() {
    run_isolated_case("production_requires_key", |_| {});
    run_isolated_case("dotenv_precedes_production_failure", |directory| {
        fs::write(
            directory.join(".env"),
            format!("APP_KEY={VALID_KEY}\nRULLST_ENV=production\n"),
        )
        .expect("dotenv fixture should be written");
    });
    run_isolated_case("toml_key_is_accepted", |directory| {
        fs::write(
            directory.join("Rullst.toml"),
            format!("app_key = \"{VALID_KEY}\"\n[app]\nenv = \"production\"\n"),
        )
        .expect("Rullst.toml fixture should be written");
    });
    run_isolated_case("process_environment_precedes_dotenv", |directory| {
        fs::write(
            directory.join(".env"),
            "APP_KEY=abcdefghijklmnopqrstuvwxyz012345\nRULLST_ENV=production\n",
        )
        .expect("dotenv fixture should be written");
    });
    run_isolated_case("toml_environment_requires_key", |directory| {
        fs::write(
            directory.join("Rullst.toml"),
            "[app]\nenv = \"production\"\n",
        )
        .expect("Rullst.toml fixture should be written");
    });
    run_isolated_case("unreadable_toml_is_reported", |directory| {
        fs::create_dir(directory.join("Rullst.toml")).expect("directory fixture should be created");
    });
    #[cfg(unix)]
    run_isolated_case("non_unicode_environment_is_rejected", |_| {});
    run_isolated_case("secure_cookie_in_production", |_| {});
    run_isolated_case("persisted_development_key_is_reused", |directory| {
        let key: Vec<u8> = (0_u8..32).collect();
        fs::write(
            directory.join(".rullst_dev_key"),
            general_purpose::STANDARD.encode(key),
        )
        .expect("development key fixture should be written");
    });
    run_isolated_case("development_key_is_created_privately", |_| {});
}

#[test]
fn documented_placeholder_app_keys_are_rejected() {
    assert!(matches!(
        validate_app_key(b"mock_0123456789abcdefghijklmnopq"),
        Err(AuthError::MissingAppKey(_))
    ));
}

#[test]
fn app_key_resolution_child() {
    let Ok(case) = std::env::var(CHILD_CASE) else {
        return;
    };

    match case.as_str() {
        "production_requires_key" => {
            // The parent removes all environment selectors; this case supplies
            // production through dotenv to exercise the same fallback parser.
            fs::write(".env", "RULLST_ENV=production\n")
                .expect("production dotenv fixture should be written");
            assert!(matches!(get_app_key(), Err(AuthError::MissingAppKey(_))));
        }
        "dotenv_precedes_production_failure"
        | "toml_key_is_accepted"
        | "process_environment_precedes_dotenv" => {
            assert_eq!(
                get_app_key().expect("configured key should resolve"),
                VALID_KEY.as_bytes()
            );
            assert_eq!(
                get_app_key().expect("configured key should be cached"),
                VALID_KEY.as_bytes()
            );
        }
        "toml_environment_requires_key" => {
            assert!(matches!(get_app_key(), Err(AuthError::MissingAppKey(_))));
        }
        "unreadable_toml_is_reported" => {
            assert!(matches!(get_app_key(), Err(AuthError::General(_))));
        }
        #[cfg(unix)]
        "non_unicode_environment_is_rejected" => {
            assert!(matches!(get_app_key(), Err(AuthError::General(_))));
        }
        "secure_cookie_in_production" => {
            let cookie = make_login_cookie(42).expect("production cookie should be created");
            assert!(cookie.contains("; Secure"));
        }
        "persisted_development_key_is_reused" => {
            assert_eq!(
                get_app_key().expect("persisted development key should resolve"),
                (0_u8..32).collect::<Vec<_>>()
            );
        }
        "development_key_is_created_privately" => {
            let generated = get_app_key().expect("development key should be generated");
            assert_eq!(generated.len(), 32);
            let encoded = fs::read_to_string(".rullst_dev_key")
                .expect("generated development key should be persisted");
            assert_eq!(
                general_purpose::STANDARD
                    .decode(encoded)
                    .expect("persisted development key should be base64"),
                generated
            );

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(".rullst_dev_key")
                    .expect("generated development key metadata should exist")
                    .permissions()
                    .mode()
                    & 0o777;
                assert_eq!(mode, 0o600);
            }
        }
        unexpected => panic!("unexpected isolated app-key case: {unexpected}"),
    }
}
