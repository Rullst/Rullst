//! The loopback policy is refused when the process environment names staging
//! or production. Its own test binary, with a single test, because it changes
//! the process environment.
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
            assert_eq!(error, NexusBuildError::LocalAccessRequiresDebugBuild);
            assert!(matches!(application, NexusAuthPolicy::Application(_)));
            assert!(!matches!(basic, Ok(NexusAuthPolicy::LoopbackOnly(_))));
            false
        }
    }
}

#[test]
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
