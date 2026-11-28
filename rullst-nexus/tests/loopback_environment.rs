//! The loopback policy is refused when the environment `Server` resolves
//! (process variables, `.env`, `[app] env`) names staging or production. Its own test binary, with a single test, because
//! it changes the process environment and the working directory.
#![allow(clippy::expect_used)]

use axum::{Router, http::request::Parts};
use rullst_nexus::{
    LocalNexusAccess, NexusAccess, NexusAuthPolicy, NexusAuthorizer, NexusAuthorizerError,
    NexusBuildError,
};

struct Admins;

impl NexusAuthorizer for Admins {
    async fn authorize(&self, _: &Parts) -> Result<NexusAccess, NexusAuthorizerError> {
        Ok(NexusAccess::SignIn)
    }
}

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

fn loopback_permitted() -> bool {
    let explicit = NexusAuthPolicy::loopback_only(LocalNexusAccess::loopback_only())
        .protect_router(Router::<()>::new());
    let application = NexusAuthPolicy::local_development_or_application(Admins);
    let basic = NexusAuthPolicy::local_development_or_basic_from_env();
    match explicit {
        Ok(_) => {
            assert!(matches!(application, NexusAuthPolicy::LoopbackOnly(_)));
            assert!(matches!(basic, Ok(NexusAuthPolicy::LoopbackOnly(_))));
            true
        }
        Err(error) => {
            // A release build is refused for being one before its environment
            // is read.
            let expected = if cfg!(debug_assertions) {
                NexusBuildError::LocalAccessRequiresDevelopmentEnvironment
            } else {
                NexusBuildError::LocalAccessRequiresDebugBuild
            };
            assert_eq!(error, expected);
            assert!(matches!(application, NexusAuthPolicy::Application(_)));
            assert!(!matches!(basic, Ok(NexusAuthPolicy::LoopbackOnly(_))));
            false
        }
    }
}

fn the_loopback_policy_follows_the_process_environment() {
    set("NEXUS_ADMIN_USERNAME", None);
    set("NEXUS_ADMIN_PASSWORD", None);
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
        assert_eq!(
            loopback_permitted(),
            permitted,
            "RULLST_ENV={rullst_env:?} APP_ENV={app_env:?}"
        );
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

/// A project name, its files and whether the loopback policy may be built.
type Case<'a> = (&'a str, &'a [(&'a str, &'a str)], bool);

fn the_loopback_policy_follows_the_dotenv_and_rullst_toml_environment() {
    let debug = cfg!(debug_assertions);
    let cases: [Case; 6] = [
        (
            "nexus-env-prod",
            &[(".env", "RULLST_ENV=production\n")],
            false,
        ),
        (
            "nexus-env-app-staging",
            &[(".env", "APP_ENV=staging\n")],
            false,
        ),
        (
            "nexus-toml-prod",
            &[("Rullst.toml", "[app]\nenv = \"production\"\n")],
            false,
        ),
        ("nexus-env-bad", &[(".env", "RULLST_ENV=bogus\n")], false),
        (
            "nexus-env-dev",
            &[(".env", "RULLST_ENV=development\n")],
            debug,
        ),
        // `.env` outranks `[app] env`, as for `Server`.
        (
            "nexus-env-over-toml",
            &[
                (".env", "RULLST_ENV=development\n"),
                ("Rullst.toml", "[app]\nenv = \"production\"\n"),
            ],
            debug,
        ),
    ];
    for (name, files, permitted) in cases {
        in_project(files, name, || {
            assert_eq!(loopback_permitted(), permitted, "{name}")
        });
    }
    // The process variable still outranks both files.
    in_project(
        &[(".env", "RULLST_ENV=production\n")],
        "nexus-process",
        || {
            set("RULLST_ENV", Some("development"));
            assert_eq!(loopback_permitted(), debug);
            set("RULLST_ENV", None);
        },
    );
}

/// One test, because both parts change the process environment and the working
/// directory.
#[test]
fn the_loopback_policy_follows_the_project_environment() {
    the_loopback_policy_follows_the_process_environment();
    the_loopback_policy_follows_the_dotenv_and_rullst_toml_environment();
}
