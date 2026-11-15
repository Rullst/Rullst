// cargo-rullst/src/generators/auth/controllers.rs — Auth controllers generator.

use crate::generators::{output_guard::write_new, register_mod_ast};
use colored::*;
use std::fs;
use std::path::Path;

const AUTH_MIDDLEWARE_TEMPLATE: &str = include_str!("auth_middleware.rs.template");
const IMPORTS_MARKER: &str = "__RULLST_MIDDLEWARE_IMPORTS__";
const EXTENSIONS_MARKER: &str = "__RULLST_MIDDLEWARE_EXTENSIONS__";
const SESSIONS_CONTROLLER_TEMPLATE: &str = include_str!("sessions_controller.rs.template");

/// `src/middlewares/auth_middleware.rs`: the session registry, sign-in and
/// the route middleware. The SaaS starter's (`billing`) also inserts the
/// signed-in account's billing identity.
pub(crate) fn render_auth_middleware(billing: bool) -> String {
    let (imports, extensions) = if billing {
        (
            "use crate::controllers::billing_controller::BillingIdentity;\n",
            "            req.extensions_mut().insert(BillingIdentity {\n                owner_id: user.id,\n                email: user.email.trim().to_ascii_lowercase(),\n            });\n",
        )
    } else {
        ("", "")
    };
    AUTH_MIDDLEWARE_TEMPLATE
        .replace(IMPORTS_MARKER, imports)
        .replace(EXTENSIONS_MARKER, extensions)
}

/// `src/controllers/sessions_controller.rs`: the "Your devices" page.
pub(crate) fn render_sessions_controller() -> String {
    SESSIONS_CONTROLLER_TEMPLATE.to_string()
}

const AUTH_CONTROLLER_TEMPLATE: &str = include_str!("auth_controller.rs.template");
const REGISTRATION_HOOK_MARKER: &str = "// __RULLST_REGISTRATION_HOOK__";

pub(crate) fn render_auth_controller(registration_hook: Option<&str>) -> String {
    AUTH_CONTROLLER_TEMPLATE.replace(REGISTRATION_HOOK_MARKER, registration_hook.unwrap_or(""))
}

pub fn generate_auth_controllers() -> Result<(), Box<dyn std::error::Error>> {
    let middlewares_dir = Path::new("src/middlewares");
    fs::create_dir_all(middlewares_dir)?;
    write_new(
        &middlewares_dir.join("auth_middleware.rs"),
        render_auth_middleware(false).as_bytes(),
    )?;
    println!("{}", "  ✨ Created 'auth_middleware' middleware.".green());
    register_mod_ast(&middlewares_dir.join("mod.rs"), "auth_middleware")?;

    let controllers_dir = Path::new("src/controllers");
    fs::create_dir_all(controllers_dir)?;
    write_new(
        &controllers_dir.join("auth_controller.rs"),
        render_auth_controller(None).as_bytes(),
    )?;
    println!("{}", "  ✨ Created 'auth_controller' controller.".green());
    register_mod_ast(&controllers_dir.join("mod.rs"), "auth_controller")?;
    write_new(
        &controllers_dir.join("sessions_controller.rs"),
        render_sessions_controller().as_bytes(),
    )?;
    println!(
        "{}",
        "  ✨ Created 'sessions_controller' (Your devices).".green()
    );
    register_mod_ast(&controllers_dir.join("mod.rs"), "sessions_controller")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // TM-DEPLOY-06: generated auth keeps blocking password work off the executor
    // and bounds how much of it runs at once.
    fn generated_auth_is_async_query_bound_and_panic_free() {
        let source = render_auth_controller(None);
        syn::parse_file(&source).expect("auth controller must parse");
        assert!(source.contains("find_by_email"));
        assert!(source.contains("DUMMY_PASSWORD_HASH"));
        assert!(source.contains("save_with_tx"));
        assert!(source.contains("transaction.rollback()"));
        assert!(!source.contains(REGISTRATION_HOOK_MARKER));
        assert!(!source.contains("User::all()"));
        // Each Argon2id run holds ~19 MiB: the unbounded async helpers would
        // let a burst of logins exhaust memory on the blocking pool.
        assert!(!source.contains("verify_password_async"));
        assert!(!source.contains("hash_password_async"));
        assert!(source.contains("Semaphore::new(MAX_CONCURRENT_PASSWORD_WORK)"));
        assert!(source.contains("rullst::runtime::task::spawn_blocking(move || {"));
        assert!(source.contains("drop(permit);"));
        for call in [
            "rullst_auth::verify_password(",
            "rullst_auth::hash_password(",
        ] {
            let lines = source
                .lines()
                .filter(|line| line.contains(call))
                .collect::<Vec<_>>();
            assert_eq!(lines.len(), 1, "{call}");
            assert!(
                lines
                    .iter()
                    .all(|line| line.contains("run_password_work(move ||"))
            );
        }
        assert!(
            source.contains("pub async fn credential_rate_limit(request: Request, next: Next)")
        );
        assert!(source.contains("rullst::RateLimitConfig::per_minute("));
        assert!(!source.contains(".unwrap("));
        assert!(!source.contains(".expect("));
        assert!(!source.contains("panic!("));
    }

    #[test]
    fn generated_logins_are_registered_sessions() {
        let controller = render_auth_controller(None);
        // Both sign-in paths register a session; none issues a stateless
        // cookie any more.
        assert_eq!(
            controller
                .matches("match login_cookie(&user, &headers).await {")
                .count(),
            2
        );
        assert!(!controller.contains("make_login_cookie"));
        assert!(controller.contains("    start_session(user, headers).await\n"));
        assert!(controller.contains("        session_epoch: 0,\n"));
        assert!(controller.contains(
            "pub async fn logout_everywhere(Extension(user_id): Extension<i32>) -> Response {\n    match User::revoke_sessions(user_id).await {"
        ));
        // Sign-out deletes the session, so a copied cookie stops working;
        // only a storage failure keeps the cookie.
        assert!(controller.contains(
            "pub async fn logout(headers: HeaderMap) -> Response {\n    if rullst_auth::extract_session_cookie(&headers).is_some() {\n        let signed_out = match sessions().await {\n            Ok(registry) => registry.sign_out(&headers).await,"
        ));
        assert!(controller.contains("            Err(error) if error.is_signed_out() => {}\n"));
        assert!(
            controller
                .contains("(StatusCode::SERVICE_UNAVAILABLE, \"Signing out failed. Try again.\")")
        );

        for billing in [false, true] {
            let middleware = render_auth_middleware(billing);
            syn::parse_file(&middleware).expect("auth middleware must parse");
            assert!(!middleware.contains("__RULLST_"));
            assert!(middleware.contains(
                "pub type Sessions = rullst::auth::session_registry::SessionRegistryFor<rullst::orm::RullstPool>;"
            ));
            // The migration owns the table: the application only connects.
            assert!(middleware.contains("Sessions::connect(pool.clone()).await"));
            assert!(!middleware.contains("Sessions::open("));
            // Long-lived responses of a session end when it is revoked.
            assert!(middleware.contains(
                ") -> Result<impl rullst::realtime::Revalidate, SessionRegistryError> {\n    Ok(sessions().await?.revalidation(session.current()))\n}"
            ));
            assert!(middleware.contains("sessions().await?.sign_in(&login).await"));
            assert!(middleware.contains(
                "NewSession::new(user.id, epoch).with_device(DeviceSummary::from_headers(headers))"
            ));
            assert!(middleware.contains("match registry.authenticate_request(headers).await {"));
            assert!(middleware.contains("Err(error) if error.is_signed_out() => return Ok(None),"));
            assert!(middleware.contains(
                "Ok(Some(user)) if user.accepts_session(&session.claims()) => Ok(Some((user, session))),"
            ));
            assert!(middleware.contains("            req.extensions_mut().insert(session);\n"));
            assert!(!middleware.contains("decrypt_session"));
            assert!(!middleware.contains(".unwrap("));
            assert!(!middleware.contains(".expect("));
            assert_eq!(middleware.contains("BillingIdentity {"), billing);
        }
        assert!(
            render_auth_middleware(true)
                .contains("use crate::controllers::billing_controller::BillingIdentity;\nuse rullst::async_runtime")
        );
    }

    #[test]
    fn the_devices_page_revokes_only_the_signed_in_accounts_sessions() {
        let source = render_sessions_controller();
        syn::parse_file(&source).expect("sessions controller must parse");
        assert!(source.contains("registry.list_sessions(session.user_id(), now).await"));
        assert!(source.contains("registry.revoke_session(session.user_id(), &reference).await"));
        assert!(source.contains("registry.revoke_other_sessions(session.current()).await"));
        assert!(source.contains("let Ok(reference) = SessionRef::parse(&form.session) else {"));
        assert!(source.contains("HeaderValue::from_static(\"no-store\")"));
        assert!(!source.contains(".unwrap("));
        assert!(!source.contains(".expect("));
    }

    #[test]
    fn registration_stores_real_timestamps() {
        let source = render_auth_controller(None);
        // The ORM inserts every field, so '' replaced the column's
        // CURRENT_TIMESTAMP default for every self-registered account.
        assert!(!source.contains("created_at: String::new()"));
        assert!(!source.contains("updated_at: String::new()"));
        assert!(source.contains("let created_at = utc_timestamp();"));
        assert!(source.contains("updated_at: created_at,\n        created_at,\n"));
        // A new account starts an empty dirty-tracking snapshot.
        assert!(source.contains(
            "        session_epoch: 0,\n        // A new record: `save()` inserts every column.\n        snapshot: rullst::db::RowSnapshot::default(),\n    };"
        ));
        assert!(source.contains(
            "pub(crate) fn utc_timestamp() -> rullst::orm::chrono::DateTime<rullst::orm::chrono::Utc> {\n    rullst::orm::chrono::Utc::now()\n}"
        ));
        assert!(source.contains("fn registration_timestamps_are_current_utc_instants()"));
    }

    #[test]
    fn registration_lengths_match_the_form_limits() {
        let source = render_auth_controller(None);
        // Byte counts rejected non-ASCII names and passwords that the form's
        // maxlength (UTF-16 units) accepted, with a "characters" message.
        assert!(!source.contains("name.len() <= 120"));
        assert!(!source.contains("payload.password.len()"));
        assert!(source.contains("value.encode_utf16().count()"));
        assert!(source.contains("form_length(name) <= 120"));
        // The minimum counts characters; the maximum stays at the 72 bytes
        // that rullst::auth::hash_password accepts, so no accepted password
        // fails later as "Error processing password".
        assert!(
            source.contains("form_length(password) >= 12 && password.len() <= MAX_PASSWORD_BYTES")
        );
        assert!(source.contains("const MAX_PASSWORD_BYTES: usize = 72;"));
        assert!(source.contains("if !valid_password(&payload.password) {"));
        assert!(source.contains("fn length_limits_count_what_the_form_counts()"));
    }
}
