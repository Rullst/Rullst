// src/generators/auth.rs — Authentication generator.

use crate::generators::is_rullst_project;
use crate::generators::migration::regenerate_migrations_mod;
use colored::*;
use std::fs;
use std::path::Path;

pub fn scaffold_auth_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!(
        "{}",
        "🛡️  Starting scaffolding of Rullst authentication system..."
            .cyan()
            .bold()
    );

    // 1. Create User Migration
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let now = chrono::Local::now();
    let timestamp = now.format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_users_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

    let migration_template = format!(
        r##"use rullst::db::schema::{{Schema, Migration}};
use rullst::db::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::create("users", |table| {{
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {{
        Schema::drop_if_exists("users").await
    }}
}}
"##,
        file_stem = file_stem
    );
    fs::write(&migration_path, migration_template)?;
    println!("{}", "  ✨ Created 'users' table migration.".green());

    regenerate_migrations_mod()?;

    // 2. Create User Model
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("user.rs");
    let model_template = r##"use rullst::db::{Orm, FromRow};

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
    fs::write(&model_path, model_template)?;
    println!("{}", "  ✨ Created 'User' model.".green());

    let mod_models_path = models_dir.join("mod.rs");
    if !mod_models_path.exists() {
        fs::write(&mod_models_path, "")?;
    }
    let mut mod_models_content = fs::read_to_string(&mod_models_path)?;
    let mut modified = false;
    if !mod_models_content.contains("pub mod user;") {
        mod_models_content.push_str("pub mod user;\n");
        modified = true;
    }
    if modified {
        fs::write(&mod_models_path, mod_models_content)?;
    }

    // 3. Create Authentication Middleware
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
    
    // 1. Extrai o cookie de sessão criptografado
    if let Some(cookie) = rullst::auth::extract_session_cookie(headers) {
        let app_key = match rullst::auth::get_app_key() {
            Ok(key) => key,
            Err(e) => {
                eprintln!("Authentication middleware error: {}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };
        
        // 2. Descriptografa o user_id
        if let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) {
            // 3. Insere o user_id nas extensions da requisição para acesso nos controllers
            req.extensions_mut().insert(user_id);
            return next.run(req).await;
        }
    }
    
    // 4. Redirect to login if not authenticated
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

    // 4. Create HTML Pages
    let pages_dir = Path::new("src/pages");
    fs::create_dir_all(pages_dir)?;
    let pages_path = pages_dir.join("auth.rs");
    let pages_template = r##"use rullst::html;
use rullst::server::Html;

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
            if (!email || !name) {
                alert("Por favor, preencha o nome e email antes de criar a Passkey.");
                return;
            }

            const res = await fetch("/auth/passkey/register/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email, name })
            });
            if (!res.ok) throw new Error(await res.text());
            
            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            options.publicKey.user.id = bufferDecode(options.publicKey.user.id);
            if (options.publicKey.excludeCredentials) {
                for (let cred of options.publicKey.excludeCredentials) {
                    cred.id = bufferDecode(cred.id);
                }
            }

            const credential = await navigator.credentials.create({
                publicKey: options.publicKey
            });

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

            if (finishRes.ok) {
                window.location.href = "/dashboard";
            } else {
                alert("Falha ao registrar Passkey: " + await finishRes.text());
            }
        } catch (err) {
            alert("Erro: " + err.message);
        }
    }

    async function loginPasskey() {
        try {
            const email = document.getElementById("email").value;
            if (!email) {
                alert("Por favor, digite seu email para fazer login com Passkey.");
                return;
            }

            const res = await fetch("/auth/passkey/login/start", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ email })
            });
            if (!res.ok) throw new Error(await res.text());

            const options = await res.json();
            options.publicKey.challenge = bufferDecode(options.publicKey.challenge);
            if (options.publicKey.allowCredentials) {
                for (let cred of options.publicKey.allowCredentials) {
                    cred.id = bufferDecode(cred.id);
                }
            }

            const credential = await navigator.credentials.get({
                publicKey: options.publicKey
            });

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

            if (finishRes.ok) {
                window.location.href = "/dashboard";
            } else {
                alert("Falha na autenticação da Passkey: " + await finishRes.text());
            }
        } catch (err) {
            alert("Erro: " + err.message);
        }
    }
</script>"#;

pub fn auth_styles() -> &'static str {
    r#"
    body {
        background-color: #0b0f19;
        color: #f1f5f9;
        font-family: system-ui, -apple-system, sans-serif;
        margin: 0;
        display: flex;
        align-items: center;
        justify-content: center;
        min-height: 100vh;
        padding: 1rem;
        box-sizing: border-box;
    }
    .card {
        background: #111827;
        border: 1px solid #1f2937;
        border-radius: 1rem;
        padding: 2.5rem;
        width: 100%;
        max-width: 420px;
        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
        text-align: center;
    }
    h1 {
        font-size: 2rem;
        margin: 0 0 0.5rem 0;
        background: linear-gradient(135deg, #38bdf8, #818cf8);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        font-weight: 800;
    }
    p.subtitle {
        color: #64748b;
        font-size: 0.95rem;
        margin: 0 0 2rem 0;
    }
    .form-group {
        margin-bottom: 1.25rem;
        text-align: left;
    }
    label {
        display: block;
        font-size: 0.85rem;
        color: #94a3b8;
        margin-bottom: 0.5rem;
        font-weight: 500;
    }
    input[type='text'], input[type='email'], input[type='password'] {
        width: 100%;
        box-sizing: border-box;
        background: #1f2937;
        border: 1px solid #374151;
        border-radius: 0.5rem;
        padding: 0.75rem 1rem;
        color: #fff;
        font-size: 0.95rem;
        transition: border-color 0.2s, box-shadow 0.2s;
    }
    input[type='text']:focus, input[type='email']:focus, input[type='password']:focus {
        outline: none;
        border-color: #6366f1;
        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
    }
    button.btn-primary {
        width: 100%;
        background: linear-gradient(135deg, #6366f1, #4f46e5);
        color: #fff;
        border: none;
        border-radius: 0.5rem;
        padding: 0.85rem;
        font-size: 0.95rem;
        font-weight: 600;
        cursor: pointer;
        transition: transform 0.1s, opacity 0.2s;
        margin-top: 0.5rem;
    }
    button.btn-primary:hover {
        transform: translateY(-1px);
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
    }
    button.btn-primary:focus-visible {
        outline: 2px solid #3b82f6;
        outline-offset: 2px;
    }
    .divider {
        display: flex;
        align-items: center;
        color: #475569;
        font-size: 0.8rem;
        margin: 1.5rem 0;
    }
    .divider::before, .divider::after {
        content: '';
        flex: 1;
        border-bottom: 1px solid #1f2937;
    }
    .divider:not(:empty)::before { margin-right: .5em; }
    .divider:not(:empty)::after { margin-left: .5em; }
    .oauth-btn {
        width: 100%;
        background: #1f2937;
        color: #fff;
        border: 1px solid #374151;
        border-radius: 0.5rem;
        padding: 0.75rem;
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 0.5rem;
        transition: background-color 0.2s;
        text-decoration: none;
        box-sizing: border-box;
    }
    .oauth-btn:hover {
        background: #374151;
    }
    .oauth-btn:focus-visible {
        outline: 2px solid #3b82f6;
        outline-offset: 2px;
    }
    .footer-link {
        margin-top: 1.5rem;
        font-size: 0.85rem;
        color: #94a3b8;
    }
    .footer-link a {
        color: #38bdf8;
        text-decoration: none;
    }
    .footer-link a:hover {
        text-decoration: underline;
    }
    "#
}

pub fn login_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        html! {
            <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
                {err}
            </div>
        }
    } else {
        String::new()
    };

    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Login - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    { rullst::html::RawHtml(auth_styles().to_string()) }
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Bem-vindo de volta"</h1>
                    <p class="subtitle">"Log in to your Rullst account"</p>
                    
                    { rullst::html::RawHtml(error_html) }

                    <form method="post" action="/login">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label for="email">"Email"</label>
                            <input type="email" id="email" name="email" placeholder="seu@email.com" autocomplete="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label for="password">"Password"</label>
                            <input type="password" id="password" name="password" placeholder="••••••••" autocomplete="current-password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary" aria-label="Sign In" aria-busy="false">"Sign In"</button>
                    </form>

                    <button type="button" onclick="loginPasskey()" class="oauth-btn btn-passkey" style="display: none; margin-top: 1rem; background: linear-gradient(135deg, #10b981, #059669); color: white; justify-content: center; width: 100%; box-sizing: border-box;">
                        "Entrar com Passkey / Biometria 🔑"
                    </button>

                    <div class="divider">"ou continuar com"</div>

                    <a href="/auth/github/redirect" class="oauth-btn">
                        <svg aria-hidden="true" style="width: 1.25rem; height: 1.25rem; fill: currentColor;" viewBox="0 0 24 24">
                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                        </svg>
                        "GitHub"
                    </a>

                    <div class="footer-link">
                        "Don't have an account? "
                        <a href="/register">"Sign up"</a>
                    </div>
                </div>
                { rullst::html::RawHtml(PASSKEY_SCRIPT.to_string()) }
            </body>
        </html>
    })
}

pub fn register_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        html! {
            <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
                {err}
            </div>
        }
    } else {
        String::new()
    };

    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Create Account - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    { rullst::html::RawHtml(auth_styles().to_string()) }
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Crie sua conta"</h1>
                    <p class="subtitle">"Sign up and start building with Rullst"</p>
                    
                    { rullst::html::RawHtml(error_html) }

                    <form method="post" action="/register">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label for="name">"Full Name"</label>
                            <input type="text" id="name" name="name" placeholder="Your Name" required="required" />
                        </div>
                        <div class="form-group">
                            <label for="email">"Email"</label>
                            <input type="email" id="email" name="email" placeholder="seu@email.com" autocomplete="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label for="password">"Password"</label>
                            <input type="password" id="password" name="password" placeholder="Minimum 6 characters" autocomplete="new-password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary" aria-label="Register account" aria-busy="false">"Registrar"</button>
                    </form>

                    <button type="button" onclick="registerPasskey()" class="oauth-btn btn-passkey" style="display: none; margin-top: 1rem; background: linear-gradient(135deg, #10b981, #059669); color: white; justify-content: center; width: 100%; box-sizing: border-box;">
                        "Registrar com Passkey / Biometria 🔑"
                    </button>

                    <div class="footer-link">
                        "Already have an account? "
                        <a href="/login">"Sign In"</a>
                    </div>
                </div>
                { rullst::html::RawHtml(PASSKEY_SCRIPT.to_string()) }
            </body>
        </html>
    })
}

pub fn dashboard_page(user_name: &str) -> Html<String> {
    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Dashboard - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    "
                    body {
                        background-color: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        padding: 2rem;
                        box-sizing: border-box;
                    }
                    .container {
                        max-width: 800px;
                        margin: 4rem auto;
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 1rem;
                        padding: 3rem;
                        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
                        text-align: center;
                    }
                    h1 {
                        font-size: 2.5rem;
                        margin: 0 0 1rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.lead {
                        color: #94a3b8;
                        font-size: 1.15rem;
                        line-height: 1.6;
                        margin-bottom: 2rem;
                    }
                    .badge {
                        display: inline-block;
                        padding: 0.5rem 1rem;
                        background: rgba(56, 189, 248, 0.1);
                        border: 1px solid rgba(56, 189, 248, 0.2);
                        color: #38bdf8;
                        border-radius: 9999px;
                        font-weight: 600;
                        font-size: 0.85rem;
                        margin-bottom: 2rem;
                    }
                    .btn-logout {
                        background: linear-gradient(135deg, #ef4444, #dc2626);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.75rem 2rem;
                        font-size: 0.95rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                        text-decoration: none;
                    }
                    .btn-logout:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <span class="badge">"Rullst Active Authentication"</span>
                    <h1>"Hello, "{user_name}"! 👋"</h1>
                    <p class="lead">"You are in a high-performance, secure restricted area. This dashboard and its entire infrastructure were built automatically via the CLI."</p>
                    <a href="/logout" class="btn-logout">"Sign Out"</a>
                </div>
            </body>
        </html>
    })
}
"##;
    fs::write(&pages_path, pages_template)?;
    println!(
        "{}",
        "  ✨ Created HTML views in 'src/pages/auth.rs'.".green()
    );

    let mod_pages_path = pages_dir.join("mod.rs");
    if !mod_pages_path.exists() {
        fs::write(&mod_pages_path, "")?;
    }
    let mut mod_pages_content = fs::read_to_string(&mod_pages_path)?;
    if !mod_pages_content.contains("pub mod auth;") {
        mod_pages_content.push_str("pub mod auth;\n");
        fs::write(&mod_pages_path, mod_pages_content)?;
    }

    // 5. Create Auth Controller
    let controllers_dir = Path::new("src/controllers");
    let controller_path = controllers_dir.join("auth_controller.rs");
    let controller_template = r##"use rullst::server::{
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
    if payload.password.len() > 72 {
        return auth::login_page(&token, Some("Password must be at most 72 characters")).into_response();
    }
    
    let users = match User::all().await {
        Ok(u) => u,
        Err(_) => return auth::login_page(&token, Some("Internal error fetching user")).into_response(),
    };
    
    let user = users.into_iter().find(|u| u.email == payload.email);
    
    let (hash, user_found) = match user.as_ref() {
        Some(u) => (u.password_hash.clone().unwrap_or_default(), true),
        None => (
            // Dummy hash to prevent timing attacks when the user is not found
            "$argon2id$v=19$m=19456,t=2,p=1$VE9CZ2d5dHVyWldOajNXZA$M0zU6o5hE/R6B+nJ9hX8+A".to_string(),
            false,
        ),
    };

    let valid_password = rullst_auth::verify_password(&payload.password, &hash);

    if !user_found || !valid_password {
        return auth::login_page(&token, Some("Incorrect email or password")).into_response();
    }

    let u = user.unwrap();

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
            return auth::register_page(&token, Some("This email address is already registered")).into_response();
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

    if let Err(e) = user.save().await {
        return auth::register_page(&token, Some(&format!("Error creating account: {}", e))).into_response();
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

pub async fn oauth_github_redirect() -> Response {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let redirect_url = std::env::var("GITHUB_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/auth/github/callback".to_string());
    
    if let Some(provider) = rullst_connect::Connect::driver("github", client_id, String::new(), redirect_url) {
        return Redirect::to(&provider.redirect_url()).into_response();
    }
    
    Redirect::to("/login").into_response()
}

pub async fn oauth_github_callback(Query(query): Query<OAuthCallbackQuery>) -> Response {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_else(|_| "dummy_client_secret".to_string());
    let redirect_url = std::env::var("GITHUB_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/auth/github/callback".to_string());

    if let Some(provider) = rullst_connect::Connect::driver("github", client_id, client_secret, redirect_url) {
        if let Ok(social_user) = provider.get_user(&query.code).await {
            let mut existing_user = None;
            if let Ok(users) = User::all().await {
                existing_user = users.into_iter().find(|u| {
                    u.oauth_provider.as_deref() == Some("github") && u.oauth_id.as_deref() == Some(&social_user.id)
                });
            }

            let user_id = if let Some(u) = existing_user {
                u.id
            } else {
                let mut user = User {
                    id: 0,
                    name: social_user.name.clone().unwrap_or_else(|| "GitHub User".to_string()),
                    email: social_user.email.clone().unwrap_or_else(|| format!("{}@github.com", social_user.id)),
                    password_hash: None,
                    oauth_provider: Some("github".to_string()),
                    oauth_id: Some(social_user.id.clone()),
                    created_at: String::new(),
                    updated_at: String::new(),
                };
                if user.save().await.is_ok() {
                    user.id
                } else {
                    return Redirect::to("/login").into_response();
                }
            };

            if let Ok(cookie) = rullst_auth::make_login_cookie(user_id) {
                let mut res = Redirect::to("/dashboard").into_response();
                res.headers_mut().append(
                    rullst::server::header::SET_COOKIE,
                    rullst::server::HeaderValue::from_str(&cookie).unwrap()
                );
                return res;
            }
        }
    }

    Redirect::to("/login").into_response()
}

pub async fn passkey_register_start(
    rullst::server::Json(payload): rullst::server::Json<PasskeyRegisterStartDto>
) -> Response {
    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
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
            None => return (rullst::server::StatusCode::BAD_REQUEST, "Registration challenge not found").into_response(),
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

            if let Err(e) = user.save().await {
                return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save user: {}", e)).into_response();
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

            if let Err(e) = user_passkey.save().await {
                return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save passkey: {}", e)).into_response();
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
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    
    let Some(user) = existing_users.into_iter().find(|u| u.email.to_lowercase() == email_lower) else {
        return (rullst::server::StatusCode::NOT_FOUND, "User not found").into_response();
    };

    let all_passkeys = match UserPasskey::all().await {
        Ok(pk) => pk,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    
    let user_credentials: Vec<webauthn_rs::prelude::Passkey> = all_passkeys
        .into_iter()
        .filter(|pk| pk.user_id == user.id)
        .filter_map(|pk| serde_json::from_str(&pk.passkey_json).ok())
        .collect();

    if user_credentials.is_empty() {
        return (rullst::server::StatusCode::BAD_REQUEST, "No passkeys registered for this user").into_response();
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
            None => return (rullst::server::StatusCode::BAD_REQUEST, "Authentication challenge not found").into_response(),
        }
    };

    let existing_users = match User::all().await {
        Ok(u) => u,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
    };
    let Some(user) = existing_users.into_iter().find(|u| u.email.to_lowercase() == email_lower) else {
        return (rullst::server::StatusCode::NOT_FOUND, "User not found").into_response();
    };

    let mut all_passkeys = match UserPasskey::all().await {
        Ok(pk) => pk,
        Err(_) => return (rullst::server::StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response(),
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
    fs::write(&controller_path, controller_template)?;
    println!(
        "{}",
        "  ✨ Created 'src/controllers/auth_controller.rs' controller.".green()
    );

    let mod_controllers_path = controllers_dir.join("mod.rs");
    if !mod_controllers_path.exists() {
        fs::write(&mod_controllers_path, "")?;
    }
    let mut mod_controllers_content = fs::read_to_string(&mod_controllers_path)?;
    if !mod_controllers_content.contains("pub mod auth_controller;") {
        mod_controllers_content.push_str("pub mod auth_controller;\n");
        fs::write(&mod_controllers_path, mod_controllers_content)?;
    }

    // 6. Register modules in src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;

        // Register required modules if not present
        for module in &["controllers", "models", "middlewares", "pages"] {
            let declaration = format!("pub mod {};", module);
            let alt_declaration = format!("mod {};", module);
            if !main_content.contains(&declaration) && !main_content.contains(&alt_declaration) {
                main_content = format!("pub mod {};\n{}", module, main_content);
            }
        }

        // Auto-inject required dependencies in Cargo.toml if needed (like rullst-connect and webauthn-rs)
        let cargo_toml_path = Path::new("Cargo.toml");
        if cargo_toml_path.exists() {
            let mut cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
            let mut modified = false;

            if !cargo_toml_content.contains("rullst-connect") {
                let current_dir = std::env::current_dir()?;
                let sibling_path = current_dir.parent().unwrap().join("rullst-connect");
                let dep_str = if sibling_path.exists() {
                    let absolute_path = sibling_path
                        .canonicalize()?
                        .display()
                        .to_string()
                        .replace("\\", "/");
                    format!("rullst-connect = {{ path = \"{}\" }}\n", absolute_path)
                } else {
                    "rullst-connect = \"10.0.0\"\n".to_string()
                };

                if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                    cargo_toml_content.insert_str(pos + 14, &dep_str);
                    modified = true;
                    println!(
                        "{}",
                        "  ✨ Added 'rullst-connect' dependency to your Cargo.toml.".green()
                    );
                }
            }

            if !cargo_toml_content.contains("webauthn-rs") {
                let dep_str = "webauthn-rs = { version = \"0.5\", default-features = false }\n";
                if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                    cargo_toml_content.insert_str(pos + 14, dep_str);
                    modified = true;
                    println!(
                        "{}",
                        "  ✨ Added 'webauthn-rs' dependency to your Cargo.toml.".green()
                    );
                }
            }

            if modified {
                fs::write(cargo_toml_path, cargo_toml_content)?;
            }
        }

        fs::write(main_path, main_content)?;
        println!("{}", "  ✨ Injetadas declarações de módulos ('pub mod controllers/models...') no seu src/main.rs.".green());
    }

    println!(
        "\n{}",
        "🎉 Authentication system generated successfully!"
            .green()
            .bold()
    );
    println!("{}", "To complete the integration:".cyan().bold());
    println!(
        "{}",
        "  1. Register the routes below in the routes! macro of your 'src/main.rs':".cyan()
    );
    println!(
        "{}",
        r##"     get("/login" => controllers::auth_controller::login_view),
     post("/login" => controllers::auth_controller::login_submit),
     get("/register" => controllers::auth_controller::register_view),
     post("/register" => controllers::auth_controller::register_submit),
     get("/logout" => controllers::auth_controller::logout),
     get("/dashboard" => controllers::auth_controller::dashboard),
     get("/auth/github/redirect" => controllers::auth_controller::oauth_github_redirect),
     get("/auth/github/callback" => controllers::auth_controller::oauth_github_callback),
     post("/auth/passkey/register/start" => controllers::auth_controller::passkey_register_start),
     post("/auth/passkey/register/finish" => controllers::auth_controller::passkey_register_finish),
     post("/auth/passkey/login/start" => controllers::auth_controller::passkey_login_start),
     post("/auth/passkey/login/finish" => controllers::auth_controller::passkey_login_finish),
     -------------------------------------------------------------------------------------"##
            .yellow()
    );
    println!(
        "{}",
        "  2. To protect routes with a middleware, apply the layer to your router:".cyan()
    );
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "     let protected_router = routes![\n         get(\"/dashboard\" => controllers::auth_controller::dashboard)\n     ]".yellow());
    println!(
        "{}",
        "     .layer(rullst::server::from_fn(middlewares::auth_middleware::auth_middleware));"
            .yellow()
    );
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!(
        "{}",
        "  3. Aplique as proteções CSRF e Security Headers globais no seu router principal:".cyan()
    );
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "     let main_router = routes![...]\n         .layer(rullst::server::from_fn(rullst::security::csrf_middleware))\n         .layer(rullst::server::from_fn(rullst::security::headers_middleware));".yellow());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "  4. Execute as migrations:".cyan());
    println!("{}", "     $ cargo rullst db:migrate".yellow());

    Ok(())
}
