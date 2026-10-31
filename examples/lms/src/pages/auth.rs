use rullst::html;
use rullst::html::Markup;
use rullst::response::Html;

/// The layout of the sign-in, registration and password pages.
const ACCOUNT_STYLES: &str = "*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:#080b11;color:#f8fafc;font:16px system-ui,sans-serif}main{width:min(92vw,430px);padding:2rem;border:1px solid #263244;border-radius:1rem;background:#0f172a}h1{margin-top:0}label{display:grid;gap:.4rem;margin:1rem 0;color:#cbd5e1}input{padding:.8rem;border:1px solid #475569;border-radius:.5rem;background:#111827;color:white}button{width:100%;padding:.85rem;border:0;border-radius:.5rem;background:#10b981;color:#052e16;font-weight:800;cursor:pointer}a{color:#34d399}.error{padding:.75rem;border-radius:.5rem;background:#450a0a;color:#fecaca}.notice{padding:.75rem;border-radius:.5rem;background:#064e3b;color:#d1fae5}";

fn account_page(title: &str, content: Markup, csp_nonce: &str) -> Markup {
    html! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width,initial-scale=1" />
                <link rel="icon" type="image/png" href="/static/rullst.png" />
                <title>{title} " — Rullst Academy Starter"</title>
                <style nonce={csp_nonce}>{ACCOUNT_STYLES}</style>
            </head>
            <body>
                <main>
                    <p>"🎓 Rullst Academy Starter"</p>
                    <h1>{title}</h1>
                    {content}
                    <p><a href="/">"Back to catalog"</a></p>
                </main>
            </body>
        </html>
    }
}

fn alert(error: Option<&str>) -> Option<Markup> {
    error.map(|message| html! { <p class="error" role="alert">{message}</p> })
}

fn status(notice: Option<&str>) -> Option<Markup> {
    notice.map(|message| html! { <p class="notice" role="status">{message}</p> })
}

/// The new-password and confirmation fields of the change and reset forms.
fn new_password_fields() -> Markup {
    html! {
        <label>"New password"<input name="password" type="password" minlength="12" maxlength="72" autocomplete="new-password" required="required" /></label>
        <label>"Confirm new password"<input name="password_confirmation" type="password" minlength="12" maxlength="72" autocomplete="new-password" required="required" /></label>
    }
}

pub fn login_page(
    csrf_token: &str,
    error: Option<&str>,
    notice: Option<&str>,
    csp_nonce: &str,
) -> Markup {
    account_page(
        "Sign in",
        html! {
            {status(notice)}
            {alert(error)}
            <form method="post" action="/login">
                <input type="hidden" name="_token" value={csrf_token} />
                <label>"Email"<input name="email" type="email" maxlength="254" autocomplete="email" required="required" /></label>
                <label>"Password"<input name="password" type="password" minlength="12" maxlength="72" autocomplete="current-password" required="required" /></label>
                <button type="submit">"Sign in"</button>
            </form>
            <p><a href="/password/forgot">"Forgot your password?"</a></p>
            <p>"New learner? " <a href="/register">"Create an account"</a></p>
        },
        csp_nonce,
    )
}

pub fn register_page(csrf_token: &str, error: Option<&str>, csp_nonce: &str) -> Markup {
    account_page(
        "Create account",
        html! {
            {alert(error)}
            <form method="post" action="/register">
                <input type="hidden" name="_token" value={csrf_token} />
                <label>"Name"<input name="name" type="text" maxlength="120" required="required" /></label>
                <label>"Email"<input name="email" type="email" maxlength="254" autocomplete="email" required="required" /></label>
                <label>"Password"<input name="password" type="password" minlength="12" maxlength="72" autocomplete="new-password" required="required" /></label>
                <button type="submit">"Create account"</button>
            </form>
            <p>"Already registered? " <a href="/login">"Sign in"</a></p>
        },
        csp_nonce,
    )
}

pub fn forgot_password_page(
    csrf_token: &str,
    error: Option<&str>,
    notice: Option<&str>,
    csp_nonce: &str,
) -> Markup {
    account_page(
        "Reset your password",
        html! {
            {status(notice)}
            {alert(error)}
            <form method="post" action="/password/forgot">
                <input type="hidden" name="_token" value={csrf_token} />
                <label>"Email"<input name="email" type="email" maxlength="254" autocomplete="email" required="required" /></label>
                <button type="submit">"Email me a reset link"</button>
            </form>
            <p><a href="/login">"Back to sign in"</a></p>
        },
        csp_nonce,
    )
}

pub fn reset_password_page(
    csrf_token: &str,
    token: &str,
    error: Option<&str>,
    csp_nonce: &str,
) -> Markup {
    account_page(
        "Choose a new password",
        html! {
            {alert(error)}
            <form method="post" action="/password/reset">
                <input type="hidden" name="_token" value={csrf_token} />
                <input type="hidden" name="token" value={token} />
                {new_password_fields()}
                <button type="submit">"Set new password"</button>
            </form>
        },
        csp_nonce,
    )
}

pub fn change_password_page(
    csrf_token: &str,
    error: Option<&str>,
    notice: Option<&str>,
    csp_nonce: &str,
) -> Markup {
    account_page(
        "Change password",
        html! {
            {status(notice)}
            {alert(error)}
            <form method="post" action="/password/change">
                <input type="hidden" name="_token" value={csrf_token} />
                <label>"Current password"<input name="current_password" type="password" autocomplete="current-password" required="required" /></label>
                {new_password_fields()}
                <button type="submit">"Change password"</button>
            </form>
            <p><a href="/dashboard">"Back to dashboard"</a></p>
        },
        csp_nonce,
    )
}

pub fn dashboard_page(user_name: &str, csrf_token: &str, csp_nonce: &str) -> Html<String> {
    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" type="image/png" href="/static/rullst.png"><title>Learner dashboard</title><style nonce="{nonce}">body{{background:#080b11;color:#f8fafc;font:16px system-ui;padding:3rem}}main{{max-width:760px;margin:auto}}a{{color:#34d399}}button{{padding:.7rem 1rem;border:0;border-radius:.5rem;background:#dc2626;color:white;cursor:pointer}}</style></head><body><main><p>🎓 Rullst Academy Starter</p><h1>Welcome, {user}</h1><p>Your encrypted session is active. Enroll in a course from the catalog to open its lessons and save your progress.</p><p><a href="/">Browse courses</a></p><p><a href="/password/change">Change password</a></p><form method="post" action="/logout"><input type="hidden" name="_token" value="{csrf}"><button type="submit">Sign out</button></form><form method="post" action="/logout/everywhere"><input type="hidden" name="_token" value="{csrf}"><button type="submit">Sign out everywhere</button></form></main></body></html>"#,
        user = rullst::html::escape_str(user_name),
        csrf = rullst::html::escape_str(csrf_token),
        nonce = rullst::html::escape_str(csp_nonce),
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn sign_out_buttons_meet_the_aa_contrast_minimum() {
        // White on #ef4444 is 3.76:1; on #dc2626 4.83:1 (WCAG AA needs 4.5:1).
        let page = super::dashboard_page("Ada", "token", "nonce").0;
        assert!(page.contains("background:#dc2626;color:white"));
        assert!(!page.contains("#ef4444"));
    }
}
