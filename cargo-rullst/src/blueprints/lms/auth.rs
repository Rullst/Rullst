// Authentication controller and compact Academy-themed pages for the LMS starter.

pub fn get_files() -> Vec<(&'static str, String)> {
    vec![
        (
            "src/controllers/auth_controller.rs",
            school_scoped_controller(),
        ),
        ("src/pages/auth.rs", AUTH_PAGES.to_string()),
    ]
}

pub(super) fn identity_controller() -> String {
    crate::generators::auth::controllers::render_auth_controller(None)
}

fn school_scoped_controller() -> String {
    crate::generators::auth::controllers::render_auth_controller(Some(
        r#"crate::services::school_service::provision_self_registration_with_tx(
            user.id,
            &mut transaction,
        )
        .await
        .map_err(|error| rullst_orm::Error::Internal(format!(
            "default school membership provisioning failed: {error}",
        )))?;"#,
    ))
}

const AUTH_PAGES: &str = r##"use rullst::response::Html;

fn auth_page(
    title: &str,
    action: &str,
    csrf_token: &str,
    error: Option<&str>,
    include_name: bool,
    csp_nonce: &str,
) -> Html<String> {
    let error_html = error.map_or_else(String::new, |message| {
        format!(
            "<p class=\"error\">{}</p>",
            rullst::html::escape_str(message)
        )
    });
    let name_html = if include_name {
        "<label>Name<input name=\"name\" type=\"text\" maxlength=\"120\" required></label>"
    } else {
        ""
    };
    let alternate = if include_name {
        "Already registered? <a href=\"/login\">Sign in</a>"
    } else {
        "New learner? <a href=\"/register\">Create an account</a>"
    };

    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" type="image/png" href="/static/rullst.png"><title>{title} — Rullst Academy Starter</title><style nonce="{nonce}">
        *{{box-sizing:border-box}}body{{margin:0;min-height:100vh;display:grid;place-items:center;background:#080b11;color:#f8fafc;font:16px system-ui,sans-serif}}main{{width:min(92vw,430px);padding:2rem;border:1px solid #263244;border-radius:1rem;background:#0f172a}}h1{{margin-top:0}}label{{display:grid;gap:.4rem;margin:1rem 0;color:#cbd5e1}}input{{padding:.8rem;border:1px solid #475569;border-radius:.5rem;background:#111827;color:white}}button{{width:100%;padding:.85rem;border:0;border-radius:.5rem;background:#10b981;color:#052e16;font-weight:800;cursor:pointer}}a{{color:#34d399}}.error{{padding:.75rem;border-radius:.5rem;background:#450a0a;color:#fecaca}}</style></head><body><main><p>🎓 Rullst Academy Starter</p><h1>{title}</h1>{error_html}<form method="post" action="{action}"><input type="hidden" name="_token" value="{csrf}">{name_html}<label>Email<input name="email" type="email" maxlength="254" autocomplete="email" required></label><label>Password<input name="password" type="password" minlength="12" maxlength="72" autocomplete="current-password" required></label><button type="submit">{title}</button></form><p>{alternate}</p><p><a href="/">Back to catalog</a></p></main></body></html>"#,
        title = rullst::html::escape_str(title),
        action = rullst::html::escape_str(action),
        csrf = rullst::html::escape_str(csrf_token),
        nonce = rullst::html::escape_str(csp_nonce),
    ))
}

pub fn login_page(csrf_token: &str, error: Option<&str>, csp_nonce: &str) -> Html<String> {
    auth_page("Sign in", "/login", csrf_token, error, false, csp_nonce)
}

pub fn register_page(csrf_token: &str, error: Option<&str>, csp_nonce: &str) -> Html<String> {
    auth_page(
        "Create account",
        "/register",
        csrf_token,
        error,
        true,
        csp_nonce,
    )
}

pub fn dashboard_page(user_name: &str, csrf_token: &str, csp_nonce: &str) -> Html<String> {
    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" type="image/png" href="/static/rullst.png"><title>Learner dashboard</title><style nonce="{nonce}">body{{background:#080b11;color:#f8fafc;font:16px system-ui;padding:3rem}}main{{max-width:760px;margin:auto}}a{{color:#34d399}}button{{padding:.7rem 1rem;border:0;border-radius:.5rem;background:#ef4444;color:white;cursor:pointer}}</style></head><body><main><p>🎓 Rullst Academy Starter</p><h1>Welcome, {user}</h1><p>Your encrypted session is active and this starter assigned your account to its default demo school.</p><p><a href="/">Open application</a></p><form method="post" action="/logout"><input type="hidden" name="_token" value="{csrf}"><button type="submit">Sign out</button></form></main></body></html>"#,
        user = rullst::html::escape_str(user_name),
        csrf = rullst::html::escape_str(csrf_token),
        nonce = rullst::html::escape_str(csp_nonce),
    ))
}
"##;
