// cargo-rullst/src/generators/auth/views.rs — Auth pages template generator.

use colored::*;
use std::fs;
use std::path::Path;

pub fn generate_auth_views() -> Result<(), Box<dyn std::error::Error>> {
    let pages_dir = Path::new("src/pages");
    fs::create_dir_all(pages_dir)?;
    let pages_path = pages_dir.join("auth.rs");
    let pages_template = r##"use rullst::html;
use rullst::server::Html;

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
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Login - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            </head>
            <body>
                <div class="card">
                    <h1>"Welcome Back"</h1>
                    { rullst::html::RawHtml(error_html) }
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
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <title>"Register - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            </head>
            <body>
                <div class="card">
                    <h1>"Create Account"</h1>
                    { rullst::html::RawHtml(error_html) }
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
    })
}

pub fn dashboard_page(user_name: &str) -> Html<String> {
    Html(html! {
        <html>
            <body>
                <h1>"Hello, " {user_name} "!"</h1>
                <a href="/logout">"Sign Out"</a>
            </body>
        </html>
    })
}
"##;
    fs::write(&pages_path, pages_template)?;
    println!("{}", "  ✨ Created 'auth' views.".green());

    let mod_pages_path = pages_dir.join("mod.rs");
    if !mod_pages_path.exists() {
        fs::write(&mod_pages_path, "")?;
    }
    let mut mod_pages_content = fs::read_to_string(&mod_pages_path)?;
    if !mod_pages_content.contains("pub mod auth;") {
        mod_pages_content.push_str("pub mod auth;\n");
        fs::write(&mod_pages_path, mod_pages_content)?;
    }

    Ok(())
}
