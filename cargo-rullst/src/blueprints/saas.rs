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
    ];

    let router = router.route("/dashboard", rullst::routing::get(controllers::auth_controller::dashboard)
        .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware)))
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

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
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
"##;
    manifest.push((
        "src/controllers/auth_controller.rs",
        auth_controller.to_string(),
    ));

    let billing_controller = r##"use rullst::server::{
    Query,
    Redirect, IntoResponse,
    HeaderMap, StatusCode,
    Bytes,
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
                </div>
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
        <html lang="en" class="dark">
        <head>
            <meta charset="UTF-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>"Select a Plan - Rullst Billing"</title>
            <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
            <style>
                "* { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                body { background: #0b0f19; color: #f3f4f6; min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; overflow-x: hidden; position: relative; }
                .glow-bg { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(99, 102, 241, 0.15) 0%, rgba(139, 92, 246, 0.05) 50%, transparent 100%); top: -10%; left: -10%; z-index: -1; }
                .glow-bg-right { position: absolute; width: 600px; height: 600px; background: radial-gradient(circle, rgba(236, 72, 153, 0.1) 0%, rgba(99, 102, 241, 0.05) 50%, transparent 100%); bottom: -10%; right: -10%; z-index: -1; }
                .container { max-width: 1200px; margin: 0 auto; padding: 4rem 2rem; text-align: center; z-index: 1; }
                .header { margin-bottom: 3.5rem; }
                .badge { background: linear-gradient(135deg, #6366f1 0%, #a855f7 100%); color: white; padding: 0.35rem 1rem; border-radius: 9999px; font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; display: inline-block; margin-bottom: 1rem; }
                h1 { font-size: 3rem; font-weight: 700; background: linear-gradient(to right, #ffffff, #9ca3af); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 1rem; }
                .subtitle { color: #9ca3af; font-size: 1.15rem; max-width: 600px; margin: 0 auto; }
                
                .setup-banner { background: rgba(99, 102, 241, 0.1); backdrop-filter: blur(12px); border: 1px solid rgba(99, 102, 241, 0.2); border-radius: 1rem; padding: 1.5rem; margin-bottom: 3rem; max-width: 800px; margin-left: auto; margin-right: auto; display: flex; gap: 1.5rem; align-items: flex-start; text-align: left; box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2); animation: fade-in 1s ease-out; }
                @keyframes fade-in { from { opacity: 0; transform: translateY(-10px); } to { opacity: 1; transform: translateY(0); } }
                .setup-banner-icon { font-size: 2rem; }
                .setup-banner-content h4 { font-size: 1.2rem; margin-bottom: 0.5rem; color: #e0e7ff; }
                .setup-banner-content p { color: #9ca3af; line-height: 1.5; margin-bottom: 1rem; }
                .setup-banner-content pre { background: #111827; padding: 1rem; border-radius: 0.5rem; border: 1px solid #1f2937; overflow-x: auto; color: #a5b4fc; font-family: ui-monospace, monospace; font-size: 0.9rem; margin: 0; }
                
                .pricing-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 2rem; max-width: 1000px; margin: 0 auto; }
                .pricing-card { background: rgba(17, 24, 39, 0.7); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 24px; padding: 3rem 2rem; text-align: left; display: flex; flex-direction: column; transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1); position: relative; }
                .pricing-card:hover { transform: translateY(-8px); border-color: rgba(99, 102, 241, 0.4); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3); }
                .pricing-card.premium { border: 2px solid #6366f1; }
                .pricing-card.premium::after { content: 'Best Value'; position: absolute; top: -14px; right: 24px; background: #6366f1; color: white; font-size: 0.75rem; font-weight: 700; padding: 0.25rem 0.75rem; border-radius: 9999px; text-transform: uppercase; }
                .plan-name { font-size: 1.5rem; font-weight: 600; color: #ffffff; margin-bottom: 0.5rem; }
                .plan-desc { color: #9ca3af; font-size: 0.95rem; margin-bottom: 2rem; min-height: 40px; }
                .price-container { display: flex; align-items: baseline; margin-bottom: 2.5rem; }
                .currency { font-size: 1.75rem; font-weight: 600; color: #ffffff; }
                .price { font-size: 3.5rem; font-weight: 700; color: #ffffff; letter-spacing: -0.02em; }
                .period { color: #9ca3af; font-size: 1rem; margin-left: 0.5rem; }
                .features-list { list-style: none; margin-bottom: 3rem; flex-grow: 1; }
                .features-list li { display: flex; align-items: center; color: #d1d5db; font-size: 0.95rem; margin-bottom: 1rem; }
                .features-list svg { width: 20px; height: 20px; margin-right: 0.75rem; color: #10b981; flex-shrink: 0; }
                .btn-checkout { display: block; width: 100%; text-align: center; padding: 1rem; border-radius: 12px; font-weight: 600; text-decoration: none; font-size: 1rem; transition: all 0.3s; cursor: pointer; border: none; }
                .btn-checkout.primary { background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%); color: white; box-shadow: 0 4px 14px rgba(99, 102, 241, 0.4); }
                .btn-checkout.primary:hover { background: linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%); box-shadow: 0 6px 20px rgba(99, 102, 241, 0.6); }
                .btn-checkout.secondary { background: rgba(255, 255, 255, 0.08); color: white; border: 1px solid rgba(255, 255, 255, 0.1); }
                .btn-checkout.secondary:hover { background: rgba(255, 255, 255, 0.15); border-color: rgba(255, 255, 255, 0.25); }"
            </style>
        </head>
        <body>
            <div class="glow-bg"></div>
            <div class="glow-bg-right"></div>
            <div class="container">
                
                <div class="setup-banner">
                    <div class="setup-banner-icon">"🚀"</div>
                    <div class="setup-banner-content">
                        <h4>"Stripe Setup Required"</h4>
                        <p>"To enable real checkouts, create a " <code>".env"</code> " file in your project root with your API keys:"</p>
                        <pre><code>"BILLING_PROVIDER=stripe\nBILLING_API_KEY=sk_test_...\nBILLING_WEBHOOK_SECRET=whsec_..."</code></pre>
                    </div>
                </div>

                <div class="header">
                    <span class="badge">"Rullst Capital"</span>
                    <h1>"Simple, Transparent Pricing"</h1>
                    <p class="subtitle">"Choose the perfect plan to boost your application with next-gen fullstack performance."</p>
                </div>
                <div class="pricing-grid">
                    <div class="pricing-card">
                        <h2 class="plan-name">"Starter"</h2>
                        <p class="plan-desc">"For hobbyists and early-stage startup prototypes."</p>
                        <div class="price-container">
                            <span class="currency">"$"</span>
                            <span class="price">"9"</span>
                            <span class="period">"/mo"</span>
                        </div>
                        <ul class="features-list">
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Up to 5 Projects"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Standard SQLite Database"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Email Support"
                            </li>
                        </ul>
                        <a href="/billing/checkout?plan=price_starter" class="btn-checkout secondary">"Get Started"</a>
                    </div>
                    
                    <div class="pricing-card premium">
                        <h2 class="plan-name">"Pro"</h2>
                        <p class="plan-desc">"For growing apps needing production scaling and support."</p>
                        <div class="price-container">
                            <span class="currency">"$"</span>
                            <span class="price">"29"</span>
                            <span class="period">"/mo"</span>
                        </div>
                        <ul class="features-list">
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Unlimited Projects"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "PostgreSQL & SQLite Support"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Adaptive WAF & Bot Management"
                            </li>
                            <li>
                                <svg fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7"></path></svg>
                                "Priority Support (Sub-1 hour)"
                            </li>
                        </ul>
                        <a href="/billing/checkout?plan=price_pro" class="btn-checkout primary">"Go Pro"</a>
                    </div>
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
            table.string("subscription_id").not_null();
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
pub mod m20260601000002_create_subscriptions_table;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260601000000_create_users_table::MigrationImpl),
        Box::new(m20260601000002_create_subscriptions_table::MigrationImpl),
    ]
}
"##;
    manifest.push(("src/migrations/mod.rs", migrations_mod.to_string()));

    manifest
}
