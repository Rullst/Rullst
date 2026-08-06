// cargo-rullst/src/generators/auth/controllers.rs — Auth controllers generator.

use colored::*;
use std::fs;
use std::path::Path;

pub fn generate_auth_controllers() -> Result<(), Box<dyn std::error::Error>> {
    let middlewares_dir = Path::new("src/middlewares");
    fs::create_dir_all(middlewares_dir)?;
    let middleware_path = middlewares_dir.join("auth_middleware.rs");
    let middleware_template = r##"use rullst::server::{
    Request,
    Next,
    Response, Redirect, IntoResponse, StatusCode,
};

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let headers = req.headers();
    if let Some(cookie) = rullst::auth::extract_session_cookie(headers) {
        let app_key = match rullst::auth::get_app_key() {
            Ok(key) => key,
            Err(e) => {
                eprintln!("Authentication middleware error: {}", e);
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
    fs::write(&middleware_path, middleware_template)?;
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
    let controller_path = controllers_dir.join("auth_controller.rs");
    let controller_template = r##"use rullst::server::{
    Form,
    IntoResponse, Redirect, Response,
    HeaderMap,
};
use serde::Deserialize;
use crate::models::user::User;
use crate::pages::auth;
use rullst::auth as rullst_auth;

#[derive(Deserialize)]
pub struct RegisterDto {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

fn get_csrf_token(headers: &HeaderMap) -> String {
    headers.get(rullst::server::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            for cookie in cookie_str.split(';') {
                let trimmed = cookie.trim();
                if trimmed.starts_with("rullst_csrf=") {
                    return Some(trimmed["rullst_csrf=".len()..].to_string());
                }
            }
            None
        })
        .unwrap_or_default()
}

pub async fn login_view(headers: HeaderMap) -> impl IntoResponse {
    let token = get_csrf_token(&headers);
    auth::login_page(&token, None)
}

pub async fn login_submit(headers: HeaderMap, Form(payload): Form<LoginDto>) -> Response {
    let token = get_csrf_token(&headers);
    if payload.password.len() > 72 {
        return auth::login_page(&token, Some("Password must be at most 72 characters")).into_response();
    }
    let users = match User::all().await {
        Ok(u) => u,
        Err(_) => return auth::login_page(&token, Some("Internal error")).into_response(),
    };
    let user = users.into_iter().find(|u| u.email == payload.email);
    let Some(u) = user else {
        return auth::login_page(&token, Some("Incorrect email or password")).into_response();
    };

    let hash = u.password_hash.as_deref().unwrap_or("");
    if !rullst_auth::verify_password(&payload.password, hash) {
        return auth::login_page(&token, Some("Incorrect email or password")).into_response();
    }

    match rullst_auth::make_login_cookie(u.id) {
        Ok(cookie) => {
            let mut res = Redirect::to("/dashboard").into_response();
            res.headers_mut().append(
                rullst::server::header::SET_COOKIE,
                rullst::server::HeaderValue::from_str(&cookie).unwrap()
            );
            res
        }
        Err(_) => auth::login_page(&token, Some("Error starting session")).into_response(),
    }
}

pub async fn register_view(headers: HeaderMap) -> impl IntoResponse {
    let token = get_csrf_token(&headers);
    auth::register_page(&token, None)
}

pub async fn register_submit(headers: HeaderMap, Form(payload): Form<RegisterDto>) -> Response {
    let token = get_csrf_token(&headers);
    if payload.password.len() < 6 {
        return auth::register_page(&token, Some("Password must be at least 6 characters")).into_response();
    }
    if payload.password.len() > 72 {
        return auth::register_page(&token, Some("Password must be at most 72 characters")).into_response();
    }
    if let Ok(users) = User::all().await {
        if users.iter().any(|u| u.email == payload.email) {
            return auth::register_page(&token, Some("Email already registered")).into_response();
        }
    }

    let hash = match rullst_auth::hash_password(&payload.password) {
        Ok(h) => h,
        Err(_) => return auth::register_page(&token, Some("Error processing password")).into_response(),
    };

    let mut user = User {
        id: 0,
        name: payload.name,
        email: payload.email,
        password_hash: Some(hash),
        oauth_provider: None,
        oauth_id: None,
        created_at: String::new(),
        updated_at: String::new(),
    };

    if user.save().await.is_err() {
        return auth::register_page(&token, Some("Error creating account")).into_response();
    }

    match rullst_auth::make_login_cookie(user.id) {
        Ok(cookie) => {
            let mut res = Redirect::to("/dashboard").into_response();
            res.headers_mut().append(
                rullst::server::header::SET_COOKIE,
                rullst::server::HeaderValue::from_str(&cookie).unwrap()
            );
            res
        }
        Err(_) => Redirect::to("/login").into_response(),
    }
}

pub async fn logout() -> Response {
    let cookie = rullst_auth::make_logout_cookie();
    let mut res = Redirect::to("/login").into_response();
    res.headers_mut().append(
        rullst::server::header::SET_COOKIE,
        rullst::server::HeaderValue::from_str(&cookie).unwrap()
    );
    res
}

pub async fn dashboard(rullst::server::Extension(user_id): rullst::server::Extension<i32>) -> Response {
    if let Ok(users) = User::all().await {
        if let Some(user) = users.into_iter().find(|u| u.id == user_id) {
            return auth::dashboard_page(&user.name).into_response();
        }
    }
    Redirect::to("/login").into_response()
}
"##;
    fs::write(&controller_path, controller_template)?;
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
