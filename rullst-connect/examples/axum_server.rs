//! Google (OpenID Connect) and GitHub (OAuth 2.0) sign-in with Axum on the
//! store-backed login flow.
//!
//! `begin_login` keeps the OAuth `state`, the PKCE verifier and, for Google,
//! the OIDC nonce on the server and sets a short `HttpOnly` cookie holding
//! only a challenge id. `finish_login` consumes that challenge once, compares
//! the callback `state` in constant time (a forged or replayed callback is
//! refused) and exchanges the code with the stored verifier and nonce. Every
//! provider profile field is HTML-escaped before it reaches the page.
//!
//! Without `GOOGLE_CLIENT_ID`/`GITHUB_CLIENT_ID` the providers use `mock_*`
//! credentials, which need the `mock` feature:
//! `cargo run -p rullst-connect --example axum_server --features axum,mock`.
//! The mock sign-in returns straight to the callback with a fixed code. Set
//! `GOOGLE_CLIENT_ID`/`GOOGLE_CLIENT_SECRET` and
//! `GITHUB_CLIENT_ID`/`GITHUB_CLIENT_SECRET` to use the real providers.
//!
//! `MemoryChallengeStore` keeps challenges in this process only; use
//! `SqlChallengeStore` when several instances can receive the callback. After
//! a successful callback, a real application maps the provider subject to its
//! own account and issues its own session cookie.
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use rullst_connect::challenge::{
    ChallengeCookie, LoginError, LoginOptions, MemoryChallengeStore, begin_login, finish_login,
};
use rullst_connect::extractors::AuthCallback;
use rullst_connect::provider::Provider;
use rullst_connect::providers::github::GithubProvider;
use rullst_connect::providers::google::GoogleProvider;
use std::sync::Arc;

const ORIGIN: &str = "http://localhost:3000";

type AppError = Box<dyn std::error::Error>;

/// One sign-in provider, the name its challenges carry and its cookie.
struct Login<P> {
    name: &'static str,
    provider: P,
    cookie: ChallengeCookie,
    oidc: bool,
}

struct AppState {
    store: MemoryChallengeStore,
    google: Login<GoogleProvider>,
    github: Login<GithubProvider>,
}

impl AppState {
    /// `setting(name, offline)` returns a configured value or the offline one.
    fn new(setting: impl Fn(&str, &str) -> String) -> Result<Self, AppError> {
        Ok(Self {
            store: MemoryChallengeStore::new(1_000)?,
            google: Login {
                name: "google",
                provider: GoogleProvider::try_new(
                    setting("GOOGLE_CLIENT_ID", "mock_google_client_id"),
                    setting("GOOGLE_CLIENT_SECRET", "mock_google_client_secret").into(),
                    format!("{ORIGIN}/auth/google/callback"),
                )?,
                // `false` only because this example serves plain HTTP on
                // localhost; an application passes
                // `Environment::requires_secure_defaults()`.
                cookie: ChallengeCookie::new("/auth/google", false)?,
                oidc: true,
            },
            github: Login {
                name: "github",
                provider: GithubProvider::try_new(
                    setting("GITHUB_CLIENT_ID", "mock_github_client_id"),
                    setting("GITHUB_CLIENT_SECRET", "mock_github_client_secret").into(),
                    format!("{ORIGIN}/auth/github/callback"),
                )?,
                cookie: ChallengeCookie::new("/auth/github", false)?,
                oidc: false,
            },
        })
    }
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/auth/google", get(login_google))
        .route("/auth/google/callback", get(callback_google))
        .route("/auth/github", get(login_github))
        .route("/auth/github/callback", get(callback_github))
        .with_state(Arc::new(state))
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let state =
        AppState::new(|name, offline| std::env::var(name).unwrap_or_else(|_| offline.to_string()))?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Listening on {ORIGIN}");
    axum::serve(listener, app(state)).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(
        r#"<h1>Rullst Connect Example</h1>
<p><a href="/auth/google">Sign in with Google</a></p>
<p><a href="/auth/github">Sign in with GitHub</a></p>"#,
    )
}

async fn login_google(State(app): State<Arc<AppState>>) -> Response {
    start(&app.store, &app.google).await
}

async fn callback_google(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    callback: AuthCallback,
) -> Response {
    finish(&app.store, &app.google, &headers, &callback).await
}

async fn login_github(State(app): State<Arc<AppState>>) -> Response {
    start(&app.store, &app.github).await
}

async fn callback_github(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    callback: AuthCallback,
) -> Response {
    finish(&app.store, &app.github, &headers, &callback).await
}

/// Stores a new challenge and redirects to the provider with its cookie.
async fn start<P>(store: &MemoryChallengeStore, login: &Login<P>) -> Response
where
    P: Provider + Send + Sync,
{
    let options = if login.oidc {
        LoginOptions::oidc(login.name)
    } else {
        LoginOptions::oauth(login.name)
    };
    match begin_login(store, &login.provider, options).await {
        Ok(redirect) => (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, redirect.url().to_string()),
                (header::SET_COOKIE, redirect.set_cookie(&login.cookie)),
            ],
        )
            .into_response(),
        Err(error) => page(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "<h1>Sign-in could not start</h1><p>{}</p>",
                escape(&error.to_string())
            ),
        ),
    }
}

/// Consumes the challenge named by the cookie and shows the signed-in profile.
async fn finish<P>(
    store: &MemoryChallengeStore,
    login: &Login<P>,
    headers: &HeaderMap,
    callback: &AuthCallback,
) -> Response
where
    P: Provider + Send + Sync,
{
    let challenge_id = ChallengeCookie::read(
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok()),
    );
    let outcome = finish_login(
        store,
        &login.provider,
        login.name,
        challenge_id.as_deref(),
        callback,
    )
    .await;
    let response = match outcome {
        Ok(completed) => {
            let user = completed.user();
            // Trust an email only when the provider says it is verified.
            let email = match (&user.email, user.email_verified) {
                (Some(email), Some(true)) => email.as_str(),
                _ => "(not verified)",
            };
            let avatar = user
                .avatar_url
                .as_deref()
                .filter(|url| url.starts_with("https://"))
                .map(|url| format!(r#"<img src="{}" alt="" width="96">"#, escape(url)))
                .unwrap_or_default();
            page(
                StatusCode::OK,
                format!(
                    "<h1>Welcome, {}!</h1><p>Email: {}</p><p>Provider subject: {}</p>{avatar}",
                    escape(&user.name),
                    escape(email),
                    escape(&user.id),
                ),
            )
        }
        Err(error) => {
            let status = match error {
                LoginError::Exchange(_) => StatusCode::BAD_GATEWAY,
                _ => StatusCode::BAD_REQUEST,
            };
            page(
                status,
                format!(
                    "<h1>Sign-in failed</h1><p>{}</p>",
                    escape(&error.to_string())
                ),
            )
        }
    };
    // The challenge is single-use, so its cookie goes in every case.
    ([(header::SET_COOKIE, login.cookie.clear())], response).into_response()
}

fn page(status: StatusCode, body: String) -> Response {
    (status, Html(body)).into_response()
}

/// Escapes text for an HTML element or a quoted attribute value.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#x27;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get(app: &Router, uri: &str, cookie: Option<&str>) -> Response {
        let mut request = Request::get(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        app.clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn text(response: Response) -> String {
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    /// Starts a sign-in: the callback path the mock redirects to and the
    /// `name=value` pair of the challenge cookie.
    async fn begin(app: &Router, provider: &str) -> (String, String) {
        let login = get(app, &format!("/auth/{provider}"), None).await;
        assert_eq!(login.status(), StatusCode::SEE_OTHER);
        let location = login.headers()[header::LOCATION].to_str().unwrap();
        let callback = location.strip_prefix(ORIGIN).unwrap().to_string();
        let set_cookie = login.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(set_cookie.contains("HttpOnly"));
        let cookie = set_cookie.split(';').next().unwrap().to_string();
        (callback, cookie)
    }

    #[tokio::test]
    async fn a_callback_signs_in_only_with_its_own_unused_challenge() {
        let app = app(AppState::new(|_, offline| offline.to_string()).unwrap());
        for provider in ["google", "github"] {
            let (callback, cookie) = begin(&app, provider).await;
            assert!(callback.contains("state="));
            assert!(callback.contains("code_challenge="));
            assert_eq!(provider == "google", callback.contains("nonce="));
            // A callback without the browser's challenge cookie (login CSRF).
            let forged = get(&app, &callback, None).await;
            assert_eq!(forged.status(), StatusCode::BAD_REQUEST);

            let signed_in = get(&app, &callback, Some(&cookie)).await;
            assert_eq!(signed_in.status(), StatusCode::OK);
            assert!(text(signed_in).await.contains("Welcome, Rullst Mock User!"));
            // The challenge was consumed, so a replay is refused.
            let replayed = get(&app, &callback, Some(&cookie)).await;
            assert_eq!(replayed.status(), StatusCode::BAD_REQUEST);
        }

        // A callback whose state was swapped is refused, and consumes the challenge.
        let (callback, cookie) = begin(&app, "github").await;
        let (_, state) = callback.split_once("state=").unwrap();
        let state = state.split('&').next().unwrap();
        let tampered = callback.replace(state, "attacker-state");
        let refused = get(&app, &tampered, Some(&cookie)).await;
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
        assert!(text(refused).await.contains("state does not match"));
        let after = get(&app, &callback, Some(&cookie)).await;
        assert_eq!(after.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn profile_text_is_escaped() {
        assert_eq!(
            escape(r#"<img src=x onerror="alert('1')">&"#),
            "&lt;img src=x onerror=&quot;alert(&#x27;1&#x27;)&quot;&gt;&amp;"
        );
    }
}
