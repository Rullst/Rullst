//! Session-epoch cookie calls: reviewed in any file while no manifest
//! enables a session registry (`V13-AUTH-SESSION-REGISTRY`), and, once one
//! does, because the registry refuses what they read or issue
//! (`V13-AUTH-SESSION-EPOCH-CALLS`, #825).

use super::super::files::manifest_findings;
use super::files::plan;
use super::*;

const PACKAGE: &str = "[package]\nname = \"app\"\n\n";

fn lines_of(code: &str, found: &[(&'static str, usize)]) -> Vec<usize> {
    found
        .iter()
        .filter(|(found, _)| *found == code)
        .map(|(_, line)| *line)
        .collect()
}

fn facts(manifests: &[(&str, &str)]) -> WorkspaceFacts {
    let plans: Vec<_> = manifests
        .iter()
        .map(|(path, text)| plan(PathBuf::from(path), text))
        .collect();
    manifest_findings(&plans).1
}

/// The middleware `cargo rullst auth` and the SaaS starter generated before
/// 13.0.0-alpha.6, and a sign-in of the controller.
const EPOCH_MIDDLEWARE: &str = r#"
use crate::models::user::User;
pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let Some(cookie) = rullst::auth::extract_session_cookie(req.headers()) else {
        return Redirect::to("/login").into_response();
    };
    let Ok(claims) = rullst::auth::decrypt_session_claims(&cookie, &app_key) else {
        return Redirect::to("/login").into_response();
    };
    next.run(req).await
}
fn login_cookie(user: &User, epoch: i64) -> Result<String, rullst_auth::AuthError> {
    rullst_auth::make_login_cookie_with_epoch(user.id, epoch, rullst_auth::DEFAULT_SESSION_MAX_AGE)
}
"#;

/// The registry's session lookup of a renamed middleware, a `use` of the
/// epoch helpers and an application's own function of the same name.
const RENAMED_EPOCH_FILE: &str = r#"
use rullst_auth::{decrypt_session_claims, make_login_cookie_with_epoch};
fn cookie_epoch(headers: &HeaderMap, user_id: i32) -> Option<u64> {
    let cookie = rullst_auth::extract_session_cookie(headers)?;
    let claims = decrypt_session_claims(&cookie, &key).ok()?;
    (claims.user_id == user_id).then_some(claims.epoch)
}
fn helper() -> fn(&str, &[u8]) -> Result<SessionClaims, AuthError> { decrypt_session_claims }
fn make_login_cookie(user: i32) -> String { other::make_login_cookie(user) }
"#;

#[test]
fn epoch_cookie_calls_are_reviewed_in_any_file_until_the_registry_is_enabled() {
    for file in [
        "src/middlewares/auth_middleware.rs",
        "src/controllers/auth_controller.rs",
        "src/controllers/password_controller.rs",
        "src/controllers/mfa/account.rs",
        "src/controllers/oauth/mod.rs",
        // Renamed or moved files, which only the call shows.
        "src/auth.rs",
        "src/web/session.rs",
    ] {
        let found = scan_with(EPOCH_MIDDLEWARE, file, &WorkspaceFacts::default());
        assert_eq!(
            lines_of("V13-AUTH-SESSION-REGISTRY", &found),
            [7, 13],
            "{file}"
        );
        assert_eq!(
            lines_of("V13-AUTH-SESSION-EPOCH-CALLS", &found),
            Vec::<usize>::new(),
            "{file}"
        );
    }
    // Only calls: not the `use` line, a function passed by name or another
    // function.
    let found = scan_with(
        RENAMED_EPOCH_FILE,
        "src/auth.rs",
        &WorkspaceFacts::default(),
    );
    assert_eq!(lines_of("V13-AUTH-SESSION-REGISTRY", &found), [5]);
    // Tests build epoch cookies on purpose.
    for file in ["tests/auth.rs", "src/auth_tests.rs"] {
        let found = scan_with(EPOCH_MIDDLEWARE, file, &WorkspaceFacts::default());
        assert_eq!(
            lines_of("V13-AUTH-SESSION-REGISTRY", &found),
            Vec::<usize>::new(),
            "{file}"
        );
    }
    let in_test = format!("#[cfg(test)]\nmod tests {{\n{EPOCH_MIDDLEWARE}}}\n");
    let found = scan_with(&in_test, "src/auth.rs", &WorkspaceFacts::default());
    assert_eq!(
        lines_of("V13-AUTH-SESSION-REGISTRY", &found),
        Vec::<usize>::new()
    );
    let auth_only = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.5\", features = [\"auth\", \"auth-sqlite\"] }}\n"
    );
    assert!(!facts(&[("Cargo.toml", auth_only.as_str())]).session_registry);
}

/// With a registry feature, each remaining epoch-cookie call is reviewed in
/// its place, since the registry's middleware refuses what it reads or
/// issues (#825).
#[test]
fn epoch_cookie_calls_left_beside_a_session_registry_are_reviewed() {
    for dependency in [
        r#"rullst = { version = "=13.0.0-alpha.6", features = ["auth-session-registry-sqlite"] }"#,
        r#"rullst = { version = "=13.0.0-alpha.6", features = ["auth-session-registry-mysql"] }"#,
        r#"rullst-auth = { version = "=13.0.0-alpha.6", features = ["session-registry-postgres"] }"#,
    ] {
        let manifest = format!("{PACKAGE}[dependencies]\n{dependency}\n");
        let facts = facts(&[("Cargo.toml", manifest.as_str())]);
        assert!(facts.session_registry, "{dependency}");
        for file in ["src/controllers/password_controller.rs", "src/auth.rs"] {
            let found = scan_with(EPOCH_MIDDLEWARE, file, &facts);
            assert_eq!(
                lines_of("V13-AUTH-SESSION-EPOCH-CALLS", &found),
                [7, 13],
                "{dependency} {file}"
            );
            assert_eq!(
                lines_of("V13-AUTH-SESSION-REGISTRY", &found),
                Vec::<usize>::new()
            );
        }
        let found = scan_with(RENAMED_EPOCH_FILE, "src/password.rs", &facts);
        assert_eq!(lines_of("V13-AUTH-SESSION-EPOCH-CALLS", &found), [5]);
        let found = scan_with(EPOCH_MIDDLEWARE, "tests/sessions.rs", &facts);
        assert_eq!(
            lines_of("V13-AUTH-SESSION-EPOCH-CALLS", &found),
            Vec::<usize>::new()
        );
    }
    let referenced = format!(
        "{PACKAGE}[dependencies]\nrullst = {{ version = \"=13.0.0-alpha.6\", features = [\"auth\"] }}\n\n[features]\nsessions = [\"rullst/auth-session-registry-postgres\"]\n"
    );
    assert!(facts(&[("Cargo.toml", referenced.as_str())]).session_registry);
    // The registry's own calls are not epoch-cookie calls.
    let registry = "async fn sign_in(registry: &Registry, session: &NewSession) -> Result<String, Error> {\n    let cookie = registry.sign_in(session).await?;\n    rullst_auth::make_logout_cookie();\n    Ok(cookie)\n}\n";
    let enabled = WorkspaceFacts {
        session_registry: true,
        ..WorkspaceFacts::default()
    };
    assert_eq!(
        lines_of(
            "V13-AUTH-SESSION-EPOCH-CALLS",
            &scan_with(registry, "src/auth.rs", &enabled)
        ),
        Vec::<usize>::new()
    );
}

/// The call rule reads the last segment of the call's path, with and
/// without a registry.
#[test]
fn the_session_call_rule_names_both_epoch_functions() {
    use super::super::alpha6_gaps::session_call_rule;
    let without = WorkspaceFacts::default();
    let with = WorkspaceFacts {
        session_registry: true,
        ..WorkspaceFacts::default()
    };
    for path in [
        &["decrypt_session_claims"][..],
        &["rullst_auth", "make_login_cookie_with_epoch"][..],
        &["rullst", "auth", "decrypt_session_claims"][..],
    ] {
        assert_eq!(
            session_call_rule(path, &without).map(|rule| rule.code),
            Some("V13-AUTH-SESSION-REGISTRY"),
            "{path:?}"
        );
        assert_eq!(
            session_call_rule(path, &with).map(|rule| rule.code),
            Some("V13-AUTH-SESSION-EPOCH-CALLS"),
            "{path:?}"
        );
    }
    for path in [
        &["make_login_cookie"][..],
        &["decrypt_session_claims", "ok"][..],
        &[][..],
    ] {
        assert_eq!(session_call_rule(path, &with), None, "{path:?}");
        assert_eq!(session_call_rule(path, &without), None, "{path:?}");
    }
}
