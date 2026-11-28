//! Studio's loopback access is refused when the environment `Server` resolves
//! (process variables, `.env`, `[app] env`) names staging or production. Its own test binary, with a single test, because it
//! changes the process environment.
#![allow(clippy::expect_used)]

use axum::Router;
use rullst_studio::{LocalStudioAccess, StudioBuildError};

fn set(name: &str, value: Option<&str>) {
    // SAFETY: this binary runs a single test, so nothing reads the
    // environment concurrently.
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
}

/// The refusal for an environment that is not development or test: a release
/// build is refused for being one before its environment is read.
fn environment_refusal() -> StudioBuildError {
    if cfg!(debug_assertions) {
        StudioBuildError::LocalAccessRequiresDevelopmentEnvironment
    } else {
        StudioBuildError::LocalAccessRequiresDebugBuild
    }
}

fn local_access_follows_the_process_environment() {
    let debug = cfg!(debug_assertions);
    for (rullst_env, app_env, permitted) in [
        (None, None, debug),
        (Some("development"), None, debug),
        (Some("test"), None, debug),
        (Some("production"), None, false),
        (Some("staging"), None, false),
        (None, Some("prod"), false),
        (Some("development"), Some("production"), debug),
        // An invalid value fails closed.
        (Some("bogus"), None, false),
    ] {
        set("RULLST_ENV", rullst_env);
        set("APP_ENV", app_env);
        let built = LocalStudioAccess::loopback_only().protect_router(Router::<()>::new());
        match built {
            Ok(_) => assert!(permitted, "RULLST_ENV={rullst_env:?} APP_ENV={app_env:?}"),
            Err(error) => {
                assert_eq!(error, environment_refusal());
                assert!(!permitted, "RULLST_ENV={rullst_env:?} APP_ENV={app_env:?}");
            }
        }
    }
    set("RULLST_ENV", None);
    set("APP_ENV", None);
}

/// Runs `check` in a fresh project directory holding the given files, with the
/// process variables cleared, and restores the working directory.
fn in_project(files: &[(&str, &str)], name: &str, check: impl FnOnce()) {
    let original = std::env::current_dir().expect("working directory");
    let directory = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("project directory");
    for (file, content) in files {
        std::fs::write(directory.join(file), content).expect("project file");
    }
    set("RULLST_ENV", None);
    set("APP_ENV", None);
    std::env::set_current_dir(&directory).expect("enter project");
    check();
    std::env::set_current_dir(original).expect("leave project");
}

/// A project name, its files and the build result they must produce.
type Case<'a> = (
    &'a str,
    &'a [(&'a str, &'a str)],
    Result<(), StudioBuildError>,
);

fn built() -> Result<(), StudioBuildError> {
    LocalStudioAccess::loopback_only()
        .protect_router(Router::<()>::new())
        .map(|_| ())
}

fn local_access_follows_the_dotenv_and_rullst_toml_environment_a_server_reads() {
    let debug = cfg!(debug_assertions);
    let refused = Err(environment_refusal());
    let allowed = if debug {
        Ok(())
    } else {
        Err(StudioBuildError::LocalAccessRequiresDebugBuild)
    };
    let cases: [Case; 6] = [
        (
            "studio-env-prod",
            &[(".env", "RULLST_ENV=production\n")],
            refused,
        ),
        (
            "studio-env-app-staging",
            &[(".env", "APP_ENV=staging\n")],
            refused,
        ),
        (
            "studio-toml-prod",
            &[("Rullst.toml", "[app]\nenv = \"production\"\n")],
            refused,
        ),
        ("studio-env-bad", &[(".env", "RULLST_ENV=bogus\n")], refused),
        (
            "studio-env-dev",
            &[(".env", "RULLST_ENV=development\n")],
            allowed,
        ),
        // `.env` outranks `[app] env`, as for `Server`.
        (
            "studio-env-over-toml",
            &[
                (".env", "RULLST_ENV=development\n"),
                ("Rullst.toml", "[app]\nenv = \"production\"\n"),
            ],
            allowed,
        ),
    ];
    for (name, files, expected) in cases {
        in_project(files, name, || assert_eq!(built(), expected, "{name}"));
    }
    // The process variable still outranks both files.
    in_project(
        &[(".env", "RULLST_ENV=production\n")],
        "studio-process",
        || {
            set("RULLST_ENV", Some("development"));
            assert_eq!(built(), allowed);
            set("RULLST_ENV", None);
        },
    );
}

/// One test, because both parts change the process environment and the working
/// directory.
#[test]
fn local_access_follows_the_project_environment() {
    local_access_follows_the_process_environment();
    local_access_follows_the_dotenv_and_rullst_toml_environment_a_server_reads();
}
