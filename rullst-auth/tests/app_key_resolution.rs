use base64::{Engine as _, engine::general_purpose};
use rullst_auth::{
    AuthError, get_app_key, make_login_cookie, make_logout_cookie, parse_app_key_from_toml,
    validate_app_key,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const CHILD_CASE: &str = "RULLST_APP_KEY_TEST_CASE";
const VALID_KEY: &str = "0123456789abcdefghijklmnopqrstuv";
const DOTENV_CANARY: &str = "sk_live_dotenv_redaction_canary_7Qx2";

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
        "process_environment_precedes_dotenv"
        | "secure_cookie_in_production"
        | "process_environment_skips_malformed_dotenv" => {
            command.env("APP_KEY", VALID_KEY);
        }
        _ => {}
    }
    if matches!(
        case,
        "secure_cookie_in_production" | "process_environment_skips_malformed_dotenv"
    ) {
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
    run_isolated_case("concurrent_first_calls_agree_on_one_key", |_| {});
}

fn write_malformed_dotenv(directory: &Path) {
    // An unclosed quote makes dotenvy's parse error carry the rest of the file.
    fs::write(
        directory.join(".env"),
        format!(
            "RULLST_ENV=development
DATABASE_URL=\"postgres://unterminated\nSTRIPE_SECRET={DOTENV_CANARY}\n"
        ),
    )
    .expect("malformed dotenv fixture should be written");
}

#[test]
// TM-AUTH-02: configuration errors never echo `Rullst.toml` content.
fn malformed_rullst_toml_errors_are_redacted() {
    run_isolated_case("malformed_toml_is_redacted", |directory| {
        fs::write(
            directory.join("Rullst.toml"),
            format!("[app]\nenv = \"production\"\n[database]\nurl = \"postgres://owner:{DOTENV_CANARY}@db.example/app\n"),
        )
        .expect("malformed Rullst.toml fixture should be written");
    });
}

#[test]
// TM-AUTH-02: configuration errors never echo `.env` content.
fn malformed_dotenv_errors_are_redacted_and_skipped_when_selected_by_process() {
    run_isolated_case("malformed_dotenv_is_redacted", write_malformed_dotenv);
    run_isolated_case(
        "process_environment_skips_malformed_dotenv",
        write_malformed_dotenv,
    );
}

#[test]
fn documented_placeholder_app_keys_are_rejected() {
    assert!(matches!(
        validate_app_key(b"mock_0123456789abcdefghijklmnopq"),
        Err(AuthError::MissingAppKey(_))
    ));
}

#[test]
fn toml_app_key_parser_ignores_prefix_collisions() {
    let configured = parse_app_key_from_toml(
        "key_id = \"public-identifier-that-is-not-a-secret\"\n\
         app_key = \"actual-application-secret\"\n",
    );
    assert_eq!(
        configured.as_deref(),
        Some(b"actual-application-secret".as_slice())
    );

    let configured = parse_app_key_from_toml(
        "app_key_backup = \"stale-application-secret\"\n\
         key = \"active-application-secret\"\n",
    );
    assert_eq!(
        configured.as_deref(),
        Some(b"active-application-secret".as_slice())
    );
}

#[test]
fn legacy_toml_values_keep_existing_sessions_readable() {
    let existing_session = rullst_auth::encrypt_session(42, VALID_KEY.as_bytes())
        .expect("legacy effective key should encrypt the session");

    for name in ["app_key", "key"] {
        for suffix in ["=", "==", "=legacy-suffix"] {
            // 12.1.0 used only the value before the next '='. Changing this
            // extraction would silently rotate existing applications' keys.
            let configured =
                parse_app_key_from_toml(&format!("{name} = \"{VALID_KEY}{suffix}\"\n"))
                    .expect("legacy key field should be found");
            assert_eq!(configured, VALID_KEY.as_bytes());
            assert_eq!(
                rullst_auth::decrypt_session(&existing_session, &configured)
                    .expect("an existing session must remain readable"),
                42
            );
        }
    }
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
            assert!(make_logout_cookie().contains("; Secure"));
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
        "concurrent_first_calls_agree_on_one_key" => {
            let start = std::sync::Arc::new(std::sync::Barrier::new(16));
            let callers: Vec<_> = (0..16)
                .map(|_| {
                    let start = std::sync::Arc::clone(&start);
                    std::thread::spawn(move || {
                        start.wait();
                        get_app_key().expect("development key should resolve")
                    })
                })
                .collect();
            let keys: Vec<Vec<u8>> = callers
                .into_iter()
                .map(|caller| caller.join().expect("caller thread"))
                .collect();
            let persisted = general_purpose::STANDARD
                .decode(fs::read_to_string(".rullst_dev_key").expect("persisted key"))
                .expect("persisted development key should be base64");
            // Compare without formatting key material into assertion output.
            assert!(keys.iter().all(|key| *key == persisted));
            assert!(get_app_key().expect("cached key") == persisted);
        }
        "malformed_dotenv_is_redacted" => {
            let error = get_app_key().expect_err("malformed dotenv must fail closed");
            for rendered in [error.to_string(), format!("{error:?}")] {
                assert!(!rendered.contains(DOTENV_CANARY), "{rendered}");
                assert!(!rendered.contains("postgres://"), "{rendered}");
            }
            assert!(matches!(error, AuthError::General(message) if message.contains(".env")));
            assert!(make_logout_cookie().contains("; Secure"));
        }
        "malformed_toml_is_redacted" => {
            let key_error = get_app_key().expect_err("malformed Rullst.toml must fail closed");
            let cookie_error =
                make_login_cookie(42).expect_err("malformed Rullst.toml must fail closed");
            for error in [key_error, cookie_error] {
                for rendered in [error.to_string(), format!("{error:?}")] {
                    assert!(!rendered.contains(DOTENV_CANARY), "{rendered}");
                    assert!(!rendered.contains("postgres://"), "{rendered}");
                    assert!(rendered.contains("line 4"), "{rendered}");
                }
            }
        }
        "process_environment_skips_malformed_dotenv" => {
            // Both selectors come from the process, so `.env` is not needed.
            let cookie = make_login_cookie(42).expect("process configuration should suffice");
            assert!(cookie.contains("; Secure"));
            assert!(make_logout_cookie().contains("; Secure"));
        }
        unexpected => panic!("unexpected isolated app-key case: {unexpected}"),
    }
}
