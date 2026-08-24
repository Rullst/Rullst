// cargo-rullst/src/generators/auth/controllers.rs — Auth controllers generator.

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

pub(crate) fn auth_controller_template() -> &'static str {
    AUTH_CONTROLLER_TEMPLATE
}

pub fn generate_auth_controllers() -> Result<(), Box<dyn std::error::Error>> {
    let middlewares_dir = Path::new("src/middlewares");
    fs::create_dir_all(middlewares_dir)?;
    fs::write(
        middlewares_dir.join("auth_middleware.rs"),
        AUTH_MIDDLEWARE_TEMPLATE,
    )?;
    println!("{}", "  ✨ Created 'auth_middleware' middleware.".green());

    let mod_middlewares_path = middlewares_dir.join("mod.rs");
    if !mod_middlewares_path.exists() {
        fs::write(&mod_middlewares_path, "")?;
    }
    let mut mod_middlewares_content = fs::read_to_string(&mod_middlewares_path)?;
    if !mod_middlewares_content.contains("pub mod auth_middleware;") {
        mod_middlewares_content.push_str("pub mod auth_middleware;\n");
        fs::write(&mod_middlewares_path, mod_middlewares_content)?;
    }

    let controllers_dir = Path::new("src/controllers");
    fs::create_dir_all(controllers_dir)?;
    fs::write(
        controllers_dir.join("auth_controller.rs"),
        AUTH_CONTROLLER_TEMPLATE,
    )?;
    println!("{}", "  ✨ Created 'auth_controller' controller.".green());

    let mod_controllers_path = controllers_dir.join("mod.rs");
    if !mod_controllers_path.exists() {
        fs::write(&mod_controllers_path, "")?;
    }
    let mut mod_controllers_content = fs::read_to_string(&mod_controllers_path)?;
    if !mod_controllers_content.contains("pub mod auth_controller;") {
        mod_controllers_content.push_str("pub mod auth_controller;\n");
        fs::write(&mod_controllers_path, mod_controllers_content)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_auth_is_async_query_bound_and_panic_free() {
        syn::parse_file(AUTH_CONTROLLER_TEMPLATE).expect("auth controller must parse");
        assert!(AUTH_CONTROLLER_TEMPLATE.contains("find_by_email"));
        assert!(AUTH_CONTROLLER_TEMPLATE.contains("verify_password_async"));
        assert!(AUTH_CONTROLLER_TEMPLATE.contains("hash_password_async"));
        assert!(AUTH_CONTROLLER_TEMPLATE.contains("DUMMY_PASSWORD_HASH"));
        assert!(!AUTH_CONTROLLER_TEMPLATE.contains("User::all()"));
        assert!(!AUTH_CONTROLLER_TEMPLATE.contains("verify_password("));
        assert!(!AUTH_CONTROLLER_TEMPLATE.contains("hash_password("));
        assert!(!AUTH_CONTROLLER_TEMPLATE.contains(".unwrap("));
        assert!(!AUTH_CONTROLLER_TEMPLATE.contains(".expect("));
        assert!(!AUTH_CONTROLLER_TEMPLATE.contains("panic!("));
    }
}
