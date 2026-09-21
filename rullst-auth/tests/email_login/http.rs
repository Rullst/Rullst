// Owned test-only HTTP application. The /fixture/mail endpoint substitutes the
// private email inbox; it must never be scaffolded or mounted in a real service.
#[allow(dead_code, unused_imports)]
pub mod support;
use axum::{
    Extension, Form, Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use rullst_core::{
    config::{Environment, SecurityConfig},
    security::{CsrfToken, TenantMembership, apply_security_baseline},
};
use serde::Deserialize;
use std::sync::Arc;
use support::*;
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
struct App {
    issuer: EmailLoginService,
    redeemer: EmailLoginService,
    subject: String,
    email: String,
    browser: Arc<tokio::sync::Mutex<Option<String>>>,
}
fn cookie(headers: &HeaderMap, key: &str) -> Result<String, StatusCode> {
    let mut values = Vec::new();
    for header in headers.get_all(header::COOKIE) {
        let header = header.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
        if header.len() > 2048 {
            return Err(StatusCode::UNAUTHORIZED);
        }
        values.extend(
            header
                .split(';')
                .filter_map(|value| value.trim().split_once('='))
                .filter(|(name, _)| *name == key)
                .map(|(_, value)| value.to_owned()),
        );
    }
    if values.len() != 1 {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(values.remove(0))
}
fn fail(_: RecoveryError) -> StatusCode {
    StatusCode::UNAUTHORIZED
}
async fn start(State(app): State<App>, Extension(csrf): Extension<CsrfToken>) -> Response {
    let browser = BrowserBinding::generate().unwrap();
    *app.browser.lock().await = Some(browser.expose_cookie().to_owned());
    let mut response = Html(format!("<!doctype html><title>Email login</title><form method=post action=/request><input type=hidden name=_token value=\"{}\"><input name=email value=\"{}\"><button id=request>Send sign-in link</button></form>",csrf.as_str(),app.email)).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        format!(
            "login_browser={}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=900",
            browser.expose_cookie()
        )
        .parse()
        .unwrap(),
    );
    response
}
#[derive(Deserialize)]
struct RequestForm {
    email: String,
}
async fn request(
    State(app): State<App>,
    headers: HeaderMap,
    Form(form): Form<RequestForm>,
) -> Result<Response, StatusCode> {
    let browser = BrowserBinding::from_cookie(&cookie(&headers, "login_browser")?).map_err(fail)?;
    app.issuer
        .request_login(&form.email, &browser, &SystemEmailLoginClock)
        .await
        .map_err(fail)?;
    Ok(Html("<p id=accepted>Check your email.</p>").into_response())
}
#[derive(Deserialize)]
struct Credential {
    token: String,
}
async fn landing(
    Query(credential): Query<Credential>,
    csrf: Option<Extension<CsrfToken>>,
) -> Result<Response, StatusCode> {
    if credential.token.len() != 43
        || !credential
            .token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let csrf = csrf.as_ref().map_or("", |csrf| csrf.as_str());
    Ok(Html(format!("<!doctype html><title>Confirm sign-in</title><form method=post action=/consume><input type=hidden name=_token value=\"{csrf}\"><input type=hidden name=token value=\"{}\"><button id=confirm>Confirm sign-in</button></form>",credential.token)).into_response())
}
async fn consume(
    State(app): State<App>,
    headers: HeaderMap,
    Form(credential): Form<Credential>,
) -> Result<Response, StatusCode> {
    let browser = BrowserBinding::from_cookie(&cookie(&headers, "login_browser")?).map_err(fail)?;
    let session = app
        .redeemer
        .redeem(&credential.token, &browser, &SystemEmailLoginClock)
        .await
        .map_err(fail)?;
    let mut response = Redirect::to(session.destination()).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        format!(
            "session={}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=3600",
            session.token().expose()
        )
        .parse()
        .unwrap(),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        "login_browser=; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age=0"
            .parse()
            .unwrap(),
    );
    Ok(response)
}
async fn authorized(app: &App, headers: &HeaderMap) -> Result<String, StatusCode> {
    let token = cookie(headers, "session")?;
    let subject = app
        .issuer
        .accounts()
        .verify_session(&token, SystemEmailLoginClock.now().unwrap())
        .await
        .map_err(fail)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if subject != app.subject {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(token)
}
async fn dashboard(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Html<&'static str>, StatusCode> {
    authorized(&app, &headers).await?;
    Ok(Html(
        "<!doctype html><title>Signed in</title><p id=authenticated>Signed in</p>",
    ))
}
async fn tenant(
    State(app): State<App>,
    Path(tenant): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    authorized(&app, &headers).await?;
    TenantMembership::try_new(["school-a"])
        .unwrap()
        .select(&tenant)
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn logout(State(app): State<App>, headers: HeaderMap) -> Result<StatusCode, StatusCode> {
    let token = authorized(&app, &headers).await?;
    app.issuer
        .accounts()
        .revoke_session(&token)
        .await
        .map_err(fail)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn inbox(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let browser = cookie(&headers, "login_browser")?;
    if app.browser.lock().await.as_deref() != Some(browser.as_str()) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let notice = app
        .issuer
        .claim_notice(&SystemEmailLoginClock)
        .await
        .map_err(fail)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let link = notice.expose_link();
    app.issuer
        .complete_notice(&notice, &SystemEmailLoginClock)
        .await
        .map_err(fail)?;
    Ok(Json(serde_json::json!({"link":link.as_str()})))
}

pub async fn run(url: &str) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://localhost:{}", listener.local_addr().unwrap().port());
    let namespace = unique();
    let config =
        EmailLoginConfig::for_development(namespace, format!("{origin}/login"), "/dashboard", 20)
            .unwrap();
    let issuer = EmailLoginService::initialize(url, keys(), config.clone())
        .await
        .unwrap();
    let clock = Clock::new();
    clock.set(SystemEmailLoginClock.now().unwrap());
    let (subject, email, _) = account(&issuer, &clock).await;
    let redeemer = EmailLoginService::connect(url, keys(), config)
        .await
        .unwrap();
    let app = App {
        issuer: issuer.clone(),
        redeemer: redeemer.clone(),
        subject,
        email,
        browser: Arc::new(tokio::sync::Mutex::new(None)),
    };
    let router = Router::new()
        .route("/", get(start))
        .route("/request", post(request))
        .route("/login", get(landing))
        .route("/consume", post(consume))
        .route("/dashboard", get(dashboard))
        .route("/tenants/{tenant}", get(tenant))
        .route("/logout", post(logout))
        .route("/fixture/mail", get(inbox))
        .layer(DefaultBodyLimit::max(2048))
        .with_state(app);
    let router =
        apply_security_baseline(router, SecurityConfig::default(), Environment::Production)
            .unwrap()
            .layer(axum::middleware::map_response(
                |mut response: Response| async move {
                    response
                        .headers_mut()
                        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
                    response
                        .headers_mut()
                        .insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
                    response
                },
            ));
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let script = std::env::var_os("RULLST_EMAIL_LOGIN_BROWSER_SCRIPT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(".github/email-login-browser.mjs")
        });
    let mut child = tokio::process::Command::new("node")
        .arg(script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::json!({"origin":origin}).to_string().as_bytes())
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(100),
        child.wait_with_output(),
    )
    .await
    .unwrap()
    .unwrap();
    let _ = stop.send(());
    server.await.unwrap();
    issuer.close().await;
    redeemer.close().await;
    assert!(
        result.status.success(),
        "browser failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    println!("{}", String::from_utf8_lossy(&result.stdout));
}
