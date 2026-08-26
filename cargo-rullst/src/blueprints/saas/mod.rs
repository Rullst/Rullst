// cargo-rullst/src/blueprints/saas/mod.rs — Root of SaaS blueprint module.

pub mod billing;
pub mod models;
pub mod routes;

use super::common;

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();
    let is_repo = common::is_repo_mode(orm_pattern);
    let _ = frontend_engine;

    manifest.extend(routes::get_routes(
        project_name_safe,
        hot_reload,
        orm_pattern,
    ));
    manifest.extend(models::get_models_and_migrations());
    manifest.extend(billing::get_billing_pages());

    // 1. Controllers
    let auth_controller_code =
        include_str!("../../generators/auth/auth_controller.rs.template").to_string();
    manifest.push(("src/controllers/auth_controller.rs", auth_controller_code));

    let billing_controller_code = include_str!("../../generators/billing_controller.rs.template")
        .replace("__FOREIGN_KEY__", "user_id");
    manifest.push((
        "src/controllers/billing_controller.rs",
        billing_controller_code,
    ));

    let controllers_mod = "pub mod auth_controller;\npub mod billing_controller;\n";
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    // 2. Middlewares
    let auth_middleware_code = r##"use rullst::server::{
    Request,
    Next,
    Response, Redirect, IntoResponse, StatusCode,
};
use crate::controllers::billing_controller::BillingIdentity;
use crate::models::user::User;

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
            match User::find(user_id).await {
                Ok(Some(user)) => {
                    req.extensions_mut().insert(user_id);
                    req.extensions_mut().insert(BillingIdentity {
                        owner_id: user_id,
                        email: user.email.trim().to_ascii_lowercase(),
                    });
                    return next.run(req).await;
                }
                Ok(None) => {}
                Err(error) => {
                    eprintln!("Authentication user query failed: {error}");
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
            }
        }
    }
    Redirect::to("/login").into_response()
}
"##;
    manifest.push((
        "src/middlewares/auth_middleware.rs",
        auth_middleware_code.to_string(),
    ));

    let middlewares_mod = "pub mod auth_middleware;\n";
    manifest.push(("src/middlewares/mod.rs", middlewares_mod.to_string()));

    // 3. Pages Auth
    let pages_auth_code = r##"use rullst::response::Html;

pub fn login_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        format!("<div style=\"background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem;\">{}</div>", rullst::html::escape_str(err))
    } else {
        String::new()
    };

    Html(format!(
        "<!DOCTYPE html><html lang=\"en\" class=\"dark\"><head>\
         <meta charset=\"utf-8\" />\
         <title>Login &mdash; Rullst SaaS</title>\
         <link rel=\"icon\" type=\"image/png\" href=\"https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png\" />\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\
         <link href=\"https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap\" rel=\"stylesheet\" />\
         <style>\
         * {{ box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }}\
         body {{ background: #0b0f19; color: #f3f4f6; min-height: 100vh; display: flex; align-items: center; justify-content: center; }}\
         .card {{ background: rgba(15, 23, 42, 0.6); backdrop-filter: blur(12px); border: 1px solid rgba(255,255,255,0.08); border-radius: 1.5rem; padding: 2.5rem; width: 100%; max-width: 420px; text-align: center; }}\
         h1 {{ font-size: 2rem; margin-bottom: 1.5rem; font-weight: 700; }}\
         .form-group {{ margin-bottom: 1.25rem; text-align: left; }}\
         label {{ display: block; font-size: 0.85rem; color: #9ca3af; margin-bottom: 0.4rem; }}\
         input {{ width: 100%; padding: 0.75rem 1rem; border-radius: 0.5rem; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); color: #fff; font-size: 0.95rem; }}\
         input:focus {{ outline: none; border-color: #10b981; }}\
         .btn-primary {{ width: 100%; padding: 0.85rem; border-radius: 0.5rem; background: #10b981; color: #000; font-weight: 700; border: none; cursor: pointer; font-size: 1rem; margin-top: 0.5rem; }}\
         .btn-primary:hover {{ background: #34d399; }}\
         .links {{ margin-top: 1.5rem; font-size: 0.85rem; color: #9ca3af; }}\
         .links a {{ color: #10b981; text-decoration: none; }}\
         </style></head><body>\
         <div class=\"card\"><h1>Welcome Back</h1>{}\
         <form method=\"POST\" action=\"/login\">\
         <input type=\"hidden\" name=\"_token\" value=\"{}\" />\
         <div class=\"form-group\"><label>Email</label><input type=\"email\" name=\"email\" placeholder=\"you@example.com\" required /></div>\
         <div class=\"form-group\"><label>Password</label><input type=\"password\" name=\"password\" placeholder=\"••••••••\" required /></div>\
         <button type=\"submit\" class=\"btn-primary\">Sign In</button>\
         </form>\
         <div class=\"links\">Don't have an account? <a href=\"/register\">Register</a> | <a href=\"/\">Pricing</a></div>\
         </div></body></html>",
        error_html,
        rullst::html::escape_str(csrf_token)
    ))
}

pub fn register_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        format!("<div style=\"background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem;\">{}</div>", rullst::html::escape_str(err))
    } else {
        String::new()
    };

    Html(format!(
        "<!DOCTYPE html><html lang=\"en\" class=\"dark\"><head>\
         <meta charset=\"utf-8\" />\
         <title>Register &mdash; Rullst SaaS</title>\
         <link rel=\"icon\" type=\"image/png\" href=\"https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png\" />\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\
         <link href=\"https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap\" rel=\"stylesheet\" />\
         <style>\
         * {{ box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }}\
         body {{ background: #0b0f19; color: #f3f4f6; min-height: 100vh; display: flex; align-items: center; justify-content: center; }}\
         .card {{ background: rgba(15, 23, 42, 0.6); backdrop-filter: blur(12px); border: 1px solid rgba(255,255,255,0.08); border-radius: 1.5rem; padding: 2.5rem; width: 100%; max-width: 420px; text-align: center; }}\
         h1 {{ font-size: 2rem; margin-bottom: 1.5rem; font-weight: 700; }}\
         .form-group {{ margin-bottom: 1.25rem; text-align: left; }}\
         label {{ display: block; font-size: 0.85rem; color: #9ca3af; margin-bottom: 0.4rem; }}\
         input {{ width: 100%; padding: 0.75rem 1rem; border-radius: 0.5rem; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); color: #fff; font-size: 0.95rem; }}\
         input:focus {{ outline: none; border-color: #10b981; }}\
         .btn-primary {{ width: 100%; padding: 0.85rem; border-radius: 0.5rem; background: #10b981; color: #000; font-weight: 700; border: none; cursor: pointer; font-size: 1rem; margin-top: 0.5rem; }}\
         .btn-primary:hover {{ background: #34d399; }}\
         .links {{ margin-top: 1.5rem; font-size: 0.85rem; color: #9ca3af; }}\
         .links a {{ color: #10b981; text-decoration: none; }}\
         </style></head><body>\
         <div class=\"card\"><h1>Create Account</h1>{}\
         <form method=\"POST\" action=\"/register\">\
         <input type=\"hidden\" name=\"_token\" value=\"{}\" />\
         <div class=\"form-group\"><label>Name</label><input type=\"text\" name=\"name\" placeholder=\"John Doe\" required /></div>\
         <div class=\"form-group\"><label>Email</label><input type=\"email\" name=\"email\" placeholder=\"you@example.com\" required /></div>\
         <div class=\"form-group\"><label>Password</label><input type=\"password\" name=\"password\" placeholder=\"••••••••\" required /></div>\
         <button type=\"submit\" class=\"btn-primary\">Register</button>\
         </form>\
         <div class=\"links\">Already have an account? <a href=\"/login\">Sign In</a> | <a href=\"/\">Pricing</a></div>\
         </div></body></html>",
        error_html,
        rullst::html::escape_str(csrf_token)
    ))
}

pub fn dashboard_page(_user_name: &str) -> Html<String> {
    Html(r#"<!DOCTYPE html><html lang="en" class="dark"><head>
         <meta charset="utf-8" />
         <title>Dashboard — Rullst SaaS</title>
         <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
         <meta name="viewport" content="width=device-width, initial-scale=1.0" />
         <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
         <style>
         * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
         body { background: #0b0f19; color: #f3f4f6; min-height: 100vh; padding: 2rem; }
         .topbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 3rem; max-width: 1200px; margin: 0 auto 3rem auto; }
         .logo { font-size: 1.5rem; font-weight: 800; color: #10b981; }
         .container { max-width: 1200px; margin: 0 auto; }
         .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; margin-top: 2rem; }
         .card { background: rgba(15, 23, 42, 0.6); backdrop-filter: blur(12px); border: 1px solid rgba(255,255,255,0.08); border-radius: 1rem; padding: 2rem; }
         .btn-logout { background: #ef4444; color: white; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; font-weight: 600; font-size: 0.9rem; }
         .btn-nexus { background: #1e293b; color: white; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; font-weight: 600; font-size: 0.9rem; border: 1px solid #374151; margin-right: 0.75rem; }
         </style></head><body>
         <div class="topbar">
           <div class="logo">⚡ Rullst SaaS Dashboard</div>
           <div>
             <a href="/nexus" class="btn-nexus">⚙️ Nexus CMS</a>
             <a href="http://127.0.0.1:5555" target="_blank" class="btn-nexus">📊 Local Studio</a>
             <a href="/logout" class="btn-logout">Logout</a>
           </div>
         </div>
         <div class="container">
           <h1>Welcome to your Pro Dashboard</h1>
           <p style="color: #9ca3af; margin-top: 0.5rem;">Your account is authenticated via Argon2id password hashing and encrypted session cookies.</p>
           <div class="grid">
             <div class="card">
               <h3 style="color: #10b981; margin-bottom: 0.5rem;">💳 Subscription</h3>
               <p style="font-size: 1.5rem; font-weight: 700;">Pro Plan — Active</p>
               <p style="color: #9ca3af; font-size: 0.85rem; margin-top: 0.5rem;">Renews next month via Stripe.</p>
             </div>
             <div class="card">
               <h3 style="color: #38bdf8; margin-bottom: 0.5rem;">⚡ Performance</h3>
               <p style="font-size: 1.5rem; font-weight: 700;">&lt; 1ms Response</p>
               <p style="color: #9ca3af; font-size: 0.85rem; margin-top: 0.5rem;">Zero-bundle Rust native SSR.</p>
             </div>
             <div class="card">
               <h3 style="color: #a855f7; margin-bottom: 0.5rem;">🛡️ Security Guard</h3>
               <p style="font-size: 1.5rem; font-weight: 700;">Double-Submit CSRF</p>
               <p style="color: #9ca3af; font-size: 0.85rem; margin-top: 0.5rem;">Real-time RASP protection.</p>
             </div>
           </div>
          </div></body></html>"#.to_string())
}
"##.to_string();
    manifest.push(("src/pages/auth.rs", pages_auth_code));

    if is_repo {
        manifest.push((
            "src/repositories/user_repository.rs",
            common::generate_repository("User", "users"),
        ));
        manifest.push((
            "src/repositories/subscription_repository.rs",
            common::generate_repository("Subscription", "subscriptions"),
        ));
        manifest.push((
            "src/repositories/mod.rs",
            common::generate_repositories_mod(&["User", "Subscription"]),
        ));
    }

    manifest
}
