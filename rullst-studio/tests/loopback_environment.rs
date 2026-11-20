//! Studio's loopback access is refused when the process environment names
//! staging or production. Its own test binary, with a single test, because it
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

#[test]
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
                assert_eq!(error, StudioBuildError::LocalAccessRequiresDebugBuild);
                assert!(!permitted, "RULLST_ENV={rullst_env:?} APP_ENV={app_env:?}");
            }
        }
    }
    set("RULLST_ENV", None);
    set("APP_ENV", None);
}
