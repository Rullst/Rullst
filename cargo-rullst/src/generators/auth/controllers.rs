// cargo-rullst/src/generators/auth/controllers.rs — Auth controllers generator.

use crate::generators::{output_guard::write_new, register_mod_ast};
use colored::*;
use std::fs;
use std::path::Path;

const AUTH_MIDDLEWARE_TEMPLATE: &str = r##"use rullst::server::{
    Request,
    Next,
    Response, Redirect, IntoResponse, StatusCode,
};

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let headers = req.headers();
    if let Some(cookie) = rullst::auth::extract_session_cookie(headers) {
        let app_key = match rullst::auth::get_app_key() {
            Ok(key) => key,
            Err(error) => {
                eprintln!("Authentication middleware configuration error: {error}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        if let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) {
            req.extensions_mut().insert(user_id);
            return next.run(req).await;
        }
    }
    Redirect::to("/login").into_response()
}
"##;

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
        AUTH_MIDDLEWARE_TEMPLATE.as_bytes(),
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
    fn registration_stores_real_timestamps() {
        let source = render_auth_controller(None);
        // The ORM inserts every field, so '' replaced the column's
        // CURRENT_TIMESTAMP default for every self-registered account.
        assert!(!source.contains("created_at: String::new()"));
        assert!(!source.contains("updated_at: String::new()"));
        assert!(source.contains("let created_at = utc_timestamp();"));
        assert!(source.contains("updated_at: created_at.clone(),\n        created_at,\n"));
        assert!(source.contains("fn registration_timestamps_use_the_current_timestamp_text()"));
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
