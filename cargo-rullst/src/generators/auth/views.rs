// cargo-rullst/src/generators/auth/views.rs — Auth pages template generator.

use crate::generators::{output_guard::write_new, register_mod_ast};
use colored::*;
use std::fs;
use std::path::Path;

/// The `src/pages/auth.rs` written by `cargo rullst auth`.
pub(crate) const AUTH_VIEWS_TEMPLATE: &str = r##"use rullst::html;
use rullst::html::Markup;

fn alert(error: Option<&str>) -> Option<Markup> {
    error.map(|err| {
        html! {
            <div class="error" role="alert">
                {err}
            </div>
        }
    })
}

fn status(notice: Option<&str>) -> Option<Markup> {
    notice.map(|text| {
        html! {
            <div class="notice" role="status">
                {text}
            </div>
        }
    })
}

fn page(title: &str, content: Markup) -> Markup {
    html! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>{title} " - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            </head>
            <body>
                <div class="card">
                    {content}
                </div>
            </body>
        </html>
    }
}

pub fn login_page(
    csrf_token: &str,
    error: Option<&str>,
    notice: Option<&str>,
    _csp_nonce: &str,
) -> Markup {
    page("Login", html! {
        <h1>"Welcome Back"</h1>
        { status(notice) }
        { alert(error) }
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
        <p><a href="/password/forgot">"Forgot your password?"</a></p>
    })
}

pub fn register_page(csrf_token: &str, error: Option<&str>, _csp_nonce: &str) -> Markup {
    page("Register", html! {
        <h1>"Create Account"</h1>
        { alert(error) }
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
    })
}

/// The new-password and confirmation fields shared by the change and reset forms.
fn new_password_fields() -> Markup {
    html! {
        <div class="form-group">
            <label for="password">"New password"</label>
            <input type="password" id="password" name="password" minlength="12" maxlength="72" autocomplete="new-password" required="required" />
        </div>
        <div class="form-group">
            <label for="password_confirmation">"Confirm new password"</label>
            <input type="password" id="password_confirmation" name="password_confirmation" minlength="12" maxlength="72" autocomplete="new-password" required="required" />
        </div>
    }
}

pub fn forgot_password_page(
    csrf_token: &str,
    error: Option<&str>,
    notice: Option<&str>,
    _csp_nonce: &str,
) -> Markup {
    page("Forgot password", html! {
        <h1>"Reset your password"</h1>
        { status(notice) }
        { alert(error) }
        <form method="post" action="/password/forgot">
            <input type="hidden" name="_token" value={csrf_token} />
            <div class="form-group">
                <label for="email">"Email"</label>
                <input type="email" id="email" name="email" autocomplete="email" required="required" />
            </div>
            <button type="submit" class="btn-primary">"Email me a reset link"</button>
        </form>
        <p><a href="/login">"Back to sign in"</a></p>
    })
}

pub fn reset_password_page(
    csrf_token: &str,
    token: &str,
    error: Option<&str>,
    _csp_nonce: &str,
) -> Markup {
    page("Choose a new password", html! {
        <h1>"Choose a new password"</h1>
        { alert(error) }
        <form method="post" action="/password/reset">
            <input type="hidden" name="_token" value={csrf_token} />
            <input type="hidden" name="token" value={token} />
            { new_password_fields() }
            <button type="submit" class="btn-primary">"Set new password"</button>
        </form>
    })
}

pub fn change_password_page(
    csrf_token: &str,
    error: Option<&str>,
    notice: Option<&str>,
    _csp_nonce: &str,
) -> Markup {
    page("Change password", html! {
        <h1>"Change password"</h1>
        { status(notice) }
        { alert(error) }
        <form method="post" action="/password/change">
            <input type="hidden" name="_token" value={csrf_token} />
            <div class="form-group">
                <label for="current_password">"Current password"</label>
                <input type="password" id="current_password" name="current_password" autocomplete="current-password" required="required" />
            </div>
            { new_password_fields() }
            <button type="submit" class="btn-primary">"Change password"</button>
        </form>
        <p><a href="/dashboard">"Back to dashboard"</a></p>
    })
}

pub fn dashboard_page(user_name: &str, csrf_token: &str, _csp_nonce: &str) -> Markup {
    page("Dashboard", html! {
        <h1>"Hello, " {user_name} "!"</h1>
        <p><a href="/password/change">"Change password"</a></p>
        <form method="post" action="/logout">
            <input type="hidden" name="_token" value={csrf_token} />
            <button type="submit">"Sign Out"</button>
        </form>
        <form method="post" action="/logout/everywhere">
            <input type="hidden" name="_token" value={csrf_token} />
            <button type="submit">"Sign Out Everywhere"</button>
        </form>
    })
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
