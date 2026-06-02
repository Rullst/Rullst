// src/blueprints/saas.rs — SaaS App Starter blueprint templates.

pub fn file_manifest(_project_name_safe: &str) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    // 1. src/main.rs
    let main_rs = r##"use rullst::{routes, Server};

pub mod migrations;
pub mod models;
pub mod controllers;
pub mod middlewares;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run migrations on startup
    rullst::artisan!(crate::migrations::get_migrations());

    let router = routes![
        // Public routes
        get("/" => controllers::billing_controller::pricing_view),
        get("/pricing" => controllers::billing_controller::pricing_view),
        get("/login" => controllers::auth_controller::login_view),
        post("/login" => controllers::auth_controller::login_submit),
        get("/register" => controllers::auth_controller::register_view),
        post("/register" => controllers::auth_controller::register_submit),
        get("/logout" => controllers::auth_controller::logout),
        get("/billing/checkout" => controllers::billing_controller::checkout_redirect),
        post("/billing/webhook" => controllers::billing_controller::webhook_handler),

        // Biometrics
        post("/auth/passkey/register/start" => controllers::auth_controller::passkey_register_start),
        post("/auth/passkey/register/finish" => controllers::auth_controller::passkey_register_finish),
        post("/auth/passkey/login/start" => controllers::auth_controller::passkey_login_start),
        post("/auth/passkey/login/finish" => controllers::auth_controller::passkey_login_finish),

        // Protected routes (Dashboard)
        get("/dashboard" => controllers::auth_controller::dashboard)
            .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)),
    ]
    .layer(rullst::server::from_fn(rullst::security::csrf_middleware))
    .layer(rullst::server::from_fn(rullst::security::headers_middleware));

    println!("🚀 SaaS server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
"##.to_string();
    manifest.push(("src/main.rs", main_rs));

    // 2. Models
    let user_model = r##"use rullst::db::{Orm, RullstModel, FromRow, sqlx};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    manifest.push(("src/models/user.rs", user_model.to_string()));

    let user_passkey_model = r##"use rullst::db::{Orm, RullstModel, FromRow, sqlx};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "user_passkeys")]
pub struct UserPasskey {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub passkey_json: String,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    manifest.push(("src/models/user_passkey.rs", user_passkey_model.to_string()));

    let subscription_model = r##"use rullst::db::{Orm, RullstModel, FromRow, sqlx};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "subscriptions")]
pub struct Subscription {
    pub id: i32,
    pub user_id: i32,
    pub customer_id: String,
    pub subscription_id: String,
    pub plan_id: String,
    pub status: String,
    pub ends_at: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    manifest.push(("src/models/subscription.rs", subscription_model.to_string()));

    let models_mod = r##"pub mod user;
pub mod user_passkey;
pub mod subscription;
"##;
    manifest.push(("src/models/mod.rs", models_mod.to_string()));

    // 3. Middleware
    let auth_middleware = r##"use rullst::server::{
    Request,
    Next,
    Response, Redirect, IntoResponse,
};

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let headers = req.headers();
    if let Some(cookie) = rullst::auth::extract_session_cookie(headers) {
        let app_key = rullst::auth::get_app_key();
        if let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) {
            req.extensions_mut().insert(user_id);
            return next.run(req).await;
        }
    }
    Redirect::to("/login").into_response()
}
"##;
    manifest.push((
        "src/middlewares/auth_middleware.rs",
        auth_middleware.to_string(),
    ));

    let middlewares_mod = r##"pub mod auth_middleware;
"##;
    manifest.push(("src/middlewares/mod.rs", middlewares_mod.to_string()));

    // 4. Controllers
    // Note: Since auth_controller needs webauthn_rs, we reuse the exact controllers logic
    let auth_controller = r##"use rullst::server::{
    Form, Query,
    Html, IntoResponse, Redirect, Response,
    HeaderMap, Extension, Json, StatusCode,
    header,
};
use serde::Deserialize;
use crate::models::user::User;
use crate::models::user_passkey::UserPasskey;
use crate::pages::auth;
use rullst::auth as rullst_auth;
use rullst::auth::passkey::{PasskeyAuth, PasskeyConfig};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

static PASSKEY: LazyLock<PasskeyAuth> = LazyLock::new(|| {
    let config = PasskeyConfig::new(
        "Rullst App",
        "localhost",
        "http://localhost:3000"
    );
    PasskeyAuth::new(&config).expect("Failed to initialize PasskeyAuth")
});

static REG_STATES: LazyLock<Mutex<HashMap<String, webauthn_rs::prelude::PasskeyRegistration>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static AUTH_STATES: LazyLock<Mutex<HashMap<String, webauthn_rs::prelude::PasskeyAuthentication>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
}

#[derive(Deserialize)]
pub struct PasskeyRegisterStartDto {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct PasskeyLoginStartDto {
    pub email: String,
}

#[derive(Deserialize)]
pub struct PasskeyEmailQuery {
    pub email: String,
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

pub async fn passkey_register_start(
    rullst::server::Json(payload): rullst::server::Json<PasskeyRegisterStartDto>
) -> Response {
    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response(),
    };
    let email_lower = payload.email.to_lowercase();
    if existing_users.iter().any(|u| u.email.to_lowercase() == email_lower) {
        return (rullst::server::StatusCode::BAD_REQUEST, "Email already registered").into_response();
    }
    let next_id = existing_users.iter().map(|u| u.id).max().unwrap_or(0) + 1;
    match PASSKEY.start_register(next_id, &payload.email, &payload.name) {
        Ok((challenge, state)) => {
            if let Ok(mut states) = REG_STATES.lock() {
                states.insert(email_lower, state);
            }
            rullst::server::Json(challenge).into_response()
        }
        Err(e) => (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}

pub async fn passkey_register_finish(
    Query(query): Query<PasskeyEmailQuery>,
    rullst::server::Json(credential): rullst::server::Json<webauthn_rs::prelude::RegisterPublicKeyCredential>
) -> Response {
    let email_lower = query.email.to_lowercase();
    let state = {
        let Ok(mut states) = REG_STATES.lock() else {
            return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Lock error").into_response();
        };
        match states.remove(&email_lower) {
            Some(s) => s,
            None => return (rullst::server::StatusCode::BAD_REQUEST, "Challenge not found").into_response(),
        }
    };

    match PASSKEY.finish_register(&credential, state) {
        Ok(passkey) => {
            let name = query.email.split('@').next().unwrap_or("User").to_string();
            let mut user = User {
                id: 0,
                name,
                email: query.email.clone(),
                password_hash: None,
                oauth_provider: Some("passkey".to_string()),
                oauth_id: Some(query.email.clone()),
                created_at: String::new(),
                updated_at: String::new(),
            };

            if user.save().await.is_err() {
                return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Failed to save user").into_response();
            }

            let passkey_json = serde_json::to_string(&passkey).unwrap_or_default();
            let mut user_passkey = UserPasskey {
                id: 0,
                user_id: user.id,
                name: "Passkey".to_string(),
                passkey_json,
                created_at: String::new(),
                updated_at: String::new(),
            };

            if user_passkey.save().await.is_err() {
                return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Failed to save passkey").into_response();
            }

            match rullst_auth::make_login_cookie(user.id) {
                Ok(cookie) => {
                    let mut res = (rullst::server::StatusCode::OK, "Success").into_response();
                    res.headers_mut().append(
                        rullst::server::header::SET_COOKIE,
                        rullst::server::HeaderValue::from_str(&cookie).unwrap()
                    );
                    res
                }
                Err(_) => (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Error logging in").into_response(),
            }
        }
        Err(e) => (rullst::server::StatusCode::BAD_REQUEST, e).into_response()
    }
}

pub async fn passkey_login_start(
    rullst::server::Json(payload): rullst::server::Json<PasskeyLoginStartDto>
) -> Response {
    let email_lower = payload.email.to_lowercase();
    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response(),
    };
    let Some(user) = existing_users.into_iter().find(|u| u.email.to_lowercase() == email_lower) else {
        return (rullst::server::StatusCode::NOT_FOUND, "User not found").into_response();
    };

    let all_passkeys = match UserPasskey::all().await {
        Ok(pk) => pk,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response(),
    };
    let user_credentials: Vec<webauthn_rs::prelude::Passkey> = all_passkeys
        .into_iter()
        .filter(|pk| pk.user_id == user.id)
        .filter_map(|pk| serde_json::from_str(&pk.passkey_json).ok())
        .collect();

    if user_credentials.is_empty() {
        return (rullst::server::StatusCode::BAD_REQUEST, "No passkeys").into_response();
    }

    match PASSKEY.start_authenticate(&user_credentials) {
        Ok((challenge, state)) => {
            if let Ok(mut states) = AUTH_STATES.lock() {
                states.insert(email_lower, state);
            }
            rullst::server::Json(challenge).into_response()
        }
        Err(e) => (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
    }
}

pub async fn passkey_login_finish(
    Query(query): Query<PasskeyEmailQuery>,
    rullst::server::Json(credential): rullst::server::Json<webauthn_rs::prelude::PublicKeyCredential>
) -> Response {
    let email_lower = query.email.to_lowercase();
    let state = {
        let Ok(mut states) = AUTH_STATES.lock() else {
            return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Lock error").into_response();
        };
        match states.remove(&email_lower) {
            Some(s) => s,
            None => return (rullst::server::StatusCode::BAD_REQUEST, "Challenge not found").into_response(),
        }
    };

    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response(),
    };
    let Some(user) = existing_users.into_iter().find(|u| u.email.to_lowercase() == email_lower) else {
        return (rullst::server::StatusCode::NOT_FOUND, "User not found").into_response();
    };

    let mut all_passkeys = match UserPasskey::all().await {
        Ok(pk) => pk,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Error").into_response(),
    };
    
    let mut found_passkey = None;
    let mut found_user_passkey = None;

    for pk in all_passkeys.iter_mut() {
        if pk.user_id == user.id {
            if let Ok(parsed_pk) = serde_json::from_str::<webauthn_rs::prelude::Passkey>(&pk.passkey_json) {
                if credential.id == parsed_pk.cred_id() {
                    found_passkey = Some(parsed_pk);
                    found_user_passkey = Some(pk);
                    break;
                }
            }
        }
    }

    let (passkey, mut user_passkey) = match (found_passkey, found_user_passkey) {
        (Some(pk), Some(upk)) => (pk, upk),
        _ => return (rullst::server::StatusCode::BAD_REQUEST, "Matching credential not found").into_response(),
    };

    match PASSKEY.finish_authenticate(&credential, state, passkey) {
        Ok(updated_passkey) => {
            user_passkey.passkey_json = serde_json::to_string(&updated_passkey).unwrap_or_default();
            let _ = user_passkey.save().await;

            match rullst_auth::make_login_cookie(user.id) {
                Ok(cookie) => {
                    let mut res = (rullst::server::StatusCode::OK, "Success").into_response();
                    res.headers_mut().append(
                        rullst::server::header::SET_COOKIE,
                        rullst::server::HeaderValue::from_str(&cookie).unwrap()
                    );
                    res
                }
                Err(_) => (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Error logging in").into_response(),
            }
        }
        Err(e) => (rullst::server::StatusCode::BAD_REQUEST, e).into_response()
    }
}
"##;
    manifest.push((
        "src/controllers/auth_controller.rs",
        auth_controller.to_string(),
    ));

    let billing_controller = r##"use rullst::server::{
    Query,
    Redirect, IntoResponse,
    HeaderMap, StatusCode,
    body::Bytes,
};
use serde::Deserialize;
use std::collections::HashMap;
use rullst::capital::{BillingProvider, StripeProvider, LemonSqueezyProvider};
use rullst::db::sqlx::{self, Row};
use crate::pages::billing;

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub plan: String,
}

pub async fn pricing_view() -> impl IntoResponse {
    billing::pricing_page()
}

pub async fn checkout_redirect(Query(query): Query<CheckoutQuery>) -> impl IntoResponse {
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());
    let redirect_url = std::env::var("BILLING_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/dashboard".to_string());

    let url_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.create_checkout_session("user@example.com", &query.plan, &redirect_url).await
        }
    };

    match url_result {
        Ok(url) => Redirect::temporary(&url).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create session: {}", e)).into_response(),
    }
}

pub async fn webhook_handler(headers: HeaderMap, body: rullst::server::Bytes) -> impl IntoResponse {
    let provider_name = std::env::var("BILLING_PROVIDER").unwrap_or_else(|_| "stripe".to_string());
    let api_key = std::env::var("BILLING_API_KEY").unwrap_or_else(|_| "mock_key".to_string());
    let webhook_secret = std::env::var("BILLING_WEBHOOK_SECRET").unwrap_or_else(|_| "mock_secret".to_string());

    let mut headers_map = HashMap::new();
    for (k, v) in headers.iter() {
        if let Ok(val_str) = v.to_str() {
            headers_map.insert(k.as_str().to_string(), val_str.to_string());
        }
    }

    let event_result = match provider_name.to_lowercase().as_str() {
        "lemonsqueezy" => {
            let provider = LemonSqueezyProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
        _ => {
            let provider = StripeProvider::new(api_key, webhook_secret);
            provider.handle_webhook(&body, &headers_map)
        }
    };

    let event = match event_result {
        Ok(evt) => evt,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid signature").into_response(),
    };

    let pool = rullst_orm::Orm::pool();
    let existing = rullst::db::sqlx::query("SELECT id FROM subscriptions WHERE subscription_id = ?1")
        .bind(&event.subscription_id)
        .fetch_optional(pool)
        .await;

    match existing {
        Ok(Some(row)) => {
            let id: i32 = row.get("id");
            let _ = rullst::db::sqlx::query("UPDATE subscriptions SET status = ?1, plan_id = ?2, ends_at = ?3, updated_at = datetime('now') WHERE id = ?4")
                .bind(event.status.as_str())
                .bind(&event.plan_id)
                .bind(event.ends_at)
                .bind(id)
                .execute(pool)
                .await;
        }
        Ok(None) => {
            let _ = rullst::db::sqlx::query("INSERT INTO subscriptions (user_id, customer_id, subscription_id, plan_id, status, ends_at, created_at, updated_at) VALUES (1, ?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))")
                .bind(&event.customer_id)
                .bind(&event.subscription_id)
                .bind(&event.plan_id)
                .bind(event.status.as_str())
                .bind(event.ends_at)
                .execute(pool)
                .await;
        }
        Err(_) => {}
    }

    (StatusCode::OK, "OK").into_response()
}
"##;
    manifest.push((
        "src/controllers/billing_controller.rs",
        billing_controller.to_string(),
    ));

    let controllers_mod = r##"pub mod auth_controller;
pub mod billing_controller;
"##;
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    // 5. Pages
    // Note: Pages templates are identical to the ones in auth.rs and billing.rs
    // For brevity but complete correctness, we reuse those page strings
    // We can define the code for Pages Auth and Pages Billing here:
    // Auth Page
    let pages_auth = r##"use rullst::html;
use axum::response::Html;

const PASSKEY_SCRIPT: &str = r#"<script>
    function bufferDecode(value) {
        const base64 = value.replace(/-/g, "+").replace(/_/g, "/");
        const pad = base64.length % 4;
        const padded = pad ? base64 + "=".repeat(4 - pad) : base64;
        const binary = window.atob(padded);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }
        return bytes.buffer;
    }
    function bufferEncode(value) {
        const bytes = new Uint8Array(value);
        let binary = "";
        for (let i = 0; i < bytes.byteLength; i++) {
            binary += String.fromCharCode(bytes[i]);
        }
        const base64 = window.btoa(binary);
        return base64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
    }
    document.addEventListener("DOMContentLoaded", () => {
        if (window.PublicKeyCredential) {
            document.querySelectorAll(".btn-passkey").forEach(btn => btn.style.display = "flex");
        }
    });
    async function registerPasskey() {
        try {
            const email = document.getElementById("email").value;
            const name = document.getElementById("name").value;
            if (!email || !name) { alert("Nome e email sao obrigatorios"); return; }
            const res = await fetch("/auth/passkey/register/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, name })
            });
            if (!res.ok) throw new Error(await res.text());
            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            options.publicKey.user.id = bufferDecode(options.publicKey.user.id);
            const credential = await navigator.credentials.create({ publicKey: options.publicKey });
            const credentialJson = {
                id: credential.id,
                rawId: bufferEncode(credential.rawId),
                type: credential.type,
                response: {
                    attestationObject: bufferEncode(credential.response.attestationObject),
                    clientDataJSON: bufferEncode(credential.response.clientDataJSON),
                    transports: credential.response.getTransports ? credential.response.getTransports() : []
                }
            };
            const finishRes = await fetch("/auth/passkey/register/finish?email=" + encodeURIComponent(email), {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(credentialJson)
            });
            if (finishRes.ok) { window.location.href = "/dashboard"; } else { alert("Erro: " + await finishRes.text()); }
        } catch (err) { alert("Erro: " + err.message); }
    }
    async function loginPasskey() {
        try {
            const email = document.getElementById("email").value;
            if (!email) { alert("Email obrigatorio"); return; }
            const res = await fetch("/auth/passkey/login/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email })
            });
            if (!res.ok) throw new Error(await res.text());
            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            if (options.publicKey.allowCredentials) {
                for (let cred of options.publicKey.allowCredentials) { cred.id = bufferDecode(cred.id); }
            }
            const credential = await navigator.credentials.get({ publicKey: options.publicKey });
            const credentialJson = {
                id: credential.id,
                rawId: bufferEncode(credential.rawId),
                type: credential.type,
                response: {
                    authenticatorData: bufferEncode(credential.response.authenticatorData),
                    clientDataJSON: bufferEncode(credential.response.clientDataJSON),
                    signature: bufferEncode(credential.response.signature),
                    userHandle: credential.response.userHandle ? bufferEncode(credential.response.userHandle) : null
                }
            };
            const finishRes = await fetch("/auth/passkey/login/finish?email=" + encodeURIComponent(email), {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify(credentialJson)
            });
            if (finishRes.ok) { window.location.href = "/dashboard"; } else { alert("Erro: " + await finishRes.text()); }
        } catch (err) { alert("Erro: " + err.message); }
    }
</script>"#;

pub fn login_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = error.map(|err| html! {
        <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
            {err}
        </div>
    }).unwrap_or_default();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Login - Rullst"</title>
                <style>
                    "
                    body { background-color: #0b0f19; color: #f1f5f9; font-family: system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }
                    .card { background: #111827; border: 1px solid #1f2937; border-radius: 1rem; padding: 2.5rem; width: 100%; max-width: 420px; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5); text-align: center; }
                    h1 { font-size: 2rem; margin: 0 0 0.5rem 0; background: linear-gradient(135deg, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    .form-group { margin-bottom: 1.25rem; text-align: left; }
                    label { display: block; font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.5rem; }
                    input { width: 100%; box-sizing: border-box; background: #1f2937; border: 1px solid #374151; border-radius: 0.5rem; padding: 0.75rem 1rem; color: #fff; }
                    button.btn-primary { width: 100%; background: linear-gradient(135deg, #6366f1, #4f46e5); color: #fff; border: none; border-radius: 0.5rem; padding: 0.85rem; font-weight: 600; cursor: pointer; }
                    .oauth-btn { width: 100%; background: #1f2937; color: #fff; border: 1px solid #374151; border-radius: 0.5rem; padding: 0.75rem; font-size: 0.9rem; cursor: pointer; display: flex; align-items: center; justify-content: center; margin-top: 1rem; }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Welcome Back"</h1>
                    { rullst::html::RawHtml(error_html) }
                    <form method="post" action="/login">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label>"Email"</label>
                            <input type="email" id="email" name="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label>"Password"</label>
                            <input type="password" id="password" name="password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Sign In"</button>
                    </form>
                    <button type="button" onclick="loginPasskey()" class="oauth-btn btn-passkey" style="display: none; background: linear-gradient(135deg, #10b981, #059669);">
                        "Entrar com Passkey 🔑"
                    </button>
                </div>
                { rullst::html::RawHtml(PASSKEY_SCRIPT.to_string()) }
            </body>
        </html>
    })
}

pub fn register_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = error.map(|err| html! {
        <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
            {err}
        </div>
    }).unwrap_or_default();

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Register - Rullst"</title>
                <style>
                    "
                    body { background-color: #0b0f19; color: #f1f5f9; font-family: system-ui, sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }
                    .card { background: #111827; border: 1px solid #1f2937; border-radius: 1rem; padding: 2.5rem; width: 100%; max-width: 420px; box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5); text-align: center; }
                    h1 { font-size: 2rem; margin: 0 0 0.5rem 0; background: linear-gradient(135deg, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    .form-group { margin-bottom: 1.25rem; text-align: left; }
                    label { display: block; font-size: 0.85rem; color: #94a3b8; margin-bottom: 0.5rem; }
                    input { width: 100%; box-sizing: border-box; background: #1f2937; border: 1px solid #374151; border-radius: 0.5rem; padding: 0.75rem 1rem; color: #fff; }
                    button.btn-primary { width: 100%; background: linear-gradient(135deg, #6366f1, #4f46e5); color: #fff; border: none; border-radius: 0.5rem; padding: 0.85rem; font-weight: 600; cursor: pointer; }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Create Account"</h1>
                    { rullst::html::RawHtml(error_html) }
                    <form method="post" action="/register">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label>"Full Name"</label>
                            <input type="text" id="name" name="name" required="required" />
                        </div>
                        <div class="form-group">
                            <label>"Email"</label>
                            <input type="email" id="email" name="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label>"Password"</label>
                            <input type="password" id="password" name="password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Sign Up"</button>
                    </form>
                </div>
            </body>
        </html>
    })
}

pub fn dashboard_page(user_name: &str) -> Html<String> {
    Html(html! {
        <html>
            <head>
                <title>"Dashboard - Rullst"</title>
                <style>
                    "
                    body { background-color: #0b0f19; color: #f1f5f9; font-family: system-ui, sans-serif; padding: 4rem; text-align: center; }
                    .container { max-width: 600px; margin: 0 auto; background: #111827; padding: 3rem; border-radius: 1rem; border: 1px solid #1f2937; }
                    h1 { font-size: 2.5rem; background: linear-gradient(135deg, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    .btn-logout { display: inline-block; background: linear-gradient(135deg, #ef4444, #dc2626); color: white; padding: 0.75rem 2rem; border-radius: 0.5rem; text-decoration: none; margin-top: 2rem; }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>"Hello, " {user_name} "!"</h1>
                    <p>"Welcome to your secure Rullst SaaS Dashboard."</p>
                    <a href="/logout" class="btn-logout">"Sign Out"</a>
                </div>
            </body>
        </html>
    })
}
"##;
    manifest.push(("src/pages/auth.rs", pages_auth.to_string()));

    // Billing Page
    let pages_billing = r##"use rullst::html;
use axum::response::Html;

pub fn pricing_page() -> Html<String> {
    Html(html! {
        <!DOCTYPE html>
        <html>
        <head>
            <title>Pricing Plans - Rullst</title>
            <style>
                body { background: #0b0f19; color: #f3f4f6; font-family: system-ui, sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }
                .grid { display: flex; gap: 2rem; }
                .card { background: #111827; border: 1px solid #1f2937; border-radius: 1rem; padding: 3rem; width: 300px; text-align: left; }
                .btn { display: block; text-align: center; background: linear-gradient(135deg, #6366f1, #4f46e5); color: white; padding: 1rem; border-radius: 0.5rem; text-decoration: none; margin-top: 2rem; }
            </style>
        </head>
        <body>
            <div class="grid">
                <div class="card">
                    <h2>"Starter"</h2>
                    <h3>"$9/mo"</h3>
                    <a href="/billing/checkout?plan=price_starter" class="btn">"Choose Starter"</a>
                </div>
                <div class="card">
                    <h2>"Pro"</h2>
                    <h3>"$29/mo"</h3>
                    <a href="/billing/checkout?plan=price_pro" class="btn">"Choose Pro"</a>
                </div>
            </div>
        </body>
        </html>
    })
}
"##;
    manifest.push(("src/pages/billing.rs", pages_billing.to_string()));

    let pages_mod = r##"pub mod auth;
pub mod billing;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    // 6. Migrations
    let m1 = r##"use rullst_orm::schema::{Schema, Blueprint, Migration};
use rullst_orm::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000000_create_users_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("users", |table| {
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }).await
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("users").await
    }
}
"##;
    manifest.push((
        "src/migrations/m20260601000000_create_users_table.rs",
        m1.to_string(),
    ));

    let m2 = r##"use rullst::db::schema::{Schema, Blueprint, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000001_create_user_passkeys_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("user_passkeys", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.string("name").not_null();
            table.text("passkey_json").not_null();
            table.timestamps();
        }).await
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("user_passkeys").await
    }
}
"##;
    manifest.push((
        "src/migrations/m20260601000001_create_user_passkeys_table.rs",
        m2.to_string(),
    ));

    let m3 = r##"use rullst::db::schema::{Schema, Blueprint, Migration};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260601000002_create_subscriptions_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("subscriptions", |table| {
            table.id();
            table.integer("user_id").not_null();
            table.string("customer_id").not_null();
            table.string("subscription_id").unique().not_null();
            table.string("plan_id").not_null();
            table.string("status").not_null();
            table.integer("ends_at").nullable();
            table.timestamps();
        }).await
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("subscriptions").await
    }
}
"##;
    manifest.push((
        "src/migrations/m20260601000002_create_subscriptions_table.rs",
        m3.to_string(),
    ));

    let migrations_mod = r##"// Generated by Rullst.
pub mod m20260601000000_create_users_table;
pub mod m20260601000001_create_user_passkeys_table;
pub mod m20260601000002_create_subscriptions_table;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_users_table::MigrationImpl),
        Box::new(m20260601000001_create_user_passkeys_table::MigrationImpl),
        Box::new(m20260601000002_create_subscriptions_table::MigrationImpl),
    ]
}
"##;
    manifest.push(("src/migrations/mod.rs", migrations_mod.to_string()));

    manifest
}
