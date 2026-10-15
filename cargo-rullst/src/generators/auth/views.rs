// cargo-rullst/src/generators/auth/views.rs — Auth pages template generator.

use crate::generators::{output_guard::write_new, register_mod_ast};
use colored::*;
use std::fs;
use std::path::Path;

/// The `src/pages/auth.rs` written by `cargo rullst auth`.
pub(crate) const AUTH_VIEWS_TEMPLATE: &str = r##"use rullst::html;
use rullst::server::Html;

pub fn login_page(csrf_token: &str, error: Option<&str>, _csp_nonce: &str) -> Html<String> {
    let error_html = error.map(|err| {
        html! {
            <div class="error" role="alert">
                {err}
            </div>
        }
    });

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Login - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            </head>
            <body>
                <div class="card">
                    <h1>"Welcome Back"</h1>
                    { error_html }
                    <form method="post" action="/login">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label for="email">"Email"</label>
                            <input type="email" id="email" name="email" placeholder="you@example.com" autocomplete="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label for="password">"Password"</label>
                            <input type="password" id="password" name="password" placeholder="••••••••" autocomplete="current-password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Sign In"</button>
                    </form>
                </div>
            </body>
        </html>
    }.into_string())
}

pub fn register_page(csrf_token: &str, error: Option<&str>, _csp_nonce: &str) -> Html<String> {
    let error_html = error.map(|err| {
        html! {
            <div class="error" role="alert">
                {err}
            </div>
        }
    });

    Html(html! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Register - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            </head>
            <body>
                <div class="card">
                    <h1>"Create Account"</h1>
                    { error_html }
                    <form method="post" action="/register">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label for="name">"Name"</label>
                            <input type="text" id="name" name="name" required="required" />
                        </div>
                        <div class="form-group">
                            <label for="email">"Email"</label>
                            <input type="email" id="email" name="email" required="required" />
                        </div>
                        <div class="form-group">
                            <label for="password">"Password"</label>
                            <input type="password" id="password" name="password" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Register"</button>
                    </form>
                </div>
            </body>
        </html>
    }.into_string())
}

pub fn dashboard_page(user_name: &str, csrf_token: &str, _csp_nonce: &str) -> Html<String> {
    Html(html! {
        <html>
            <body>
                <h1>"Hello, " {user_name} "!"</h1>
                <form method="post" action="/logout">
                    <input type="hidden" name="_token" value={csrf_token} />
                    <button type="submit">"Sign Out"</button>
                </form>
                <form method="post" action="/logout/everywhere">
                    <input type="hidden" name="_token" value={csrf_token} />
                    <button type="submit">"Sign Out Everywhere"</button>
                </form>
            </body>
        </html>
    }.into_string())
}
"##;

pub fn generate_auth_views() -> Result<(), Box<dyn std::error::Error>> {
    let pages_dir = Path::new("src/pages");
    fs::create_dir_all(pages_dir)?;
    let pages_path = pages_dir.join("auth.rs");
    write_new(&pages_path, AUTH_VIEWS_TEMPLATE.as_bytes())?;
    println!("{}", "  ✨ Created 'auth' views.".green());
    register_mod_ast(&pages_dir.join("mod.rs"), "auth")?;

    Ok(())
}
