//! The smallest Google sign-in on the store-backed login flow; `axum_server`
//! adds GitHub and tests. `begin_login` keeps the state, PKCE verifier and
//! nonce on the server, and `finish_login` refuses a callback that does not
//! carry this browser's unused challenge and matching `state`.
//!
//! Without `GOOGLE_CLIENT_ID`, the provider uses `mock_*` credentials, which
//! need the `mock` feature: `cargo run -p rullst-connect --example axum_example --features axum,mock`.
//! The mock sign-in then returns straight to the callback with a fixed code.
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use rullst_connect::challenge::{
    ChallengeCookie, LoginOptions, MemoryChallengeStore, begin_login, finish_login,
};
use rullst_connect::{extractors::AuthCallback, providers::GoogleProvider};
use std::sync::Arc;

struct AppState {
    store: MemoryChallengeStore,
    google: GoogleProvider,
    cookie: ChallengeCookie,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let setting =
        |name: &str, offline: &str| std::env::var(name).unwrap_or_else(|_| offline.to_string());
    let state = Arc::new(AppState {
        store: MemoryChallengeStore::new(1_000)?,
        google: GoogleProvider::try_new(
            setting("GOOGLE_CLIENT_ID", "mock_google_client_id"),
            setting("GOOGLE_CLIENT_SECRET", "mock_google_client_secret").into(),
            "http://localhost:3000/login/callback",
        )?,
        // `false` only because this example serves plain HTTP on localhost.
        cookie: ChallengeCookie::new("/login", false)?,
    });
    let app = Router::new()
        .route("/", get(home))
        .route("/login", get(login))
        .route("/login/callback", get(callback))
        .with_state(state);

    let addr = "127.0.0.1:3000";
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn home() -> Html<&'static str> {
    Html("<h1>Rullst Connect Axum Example</h1><a href='/login'>Login with Google</a>")
}

async fn login(State(app): State<Arc<AppState>>) -> Response {
    match begin_login(&app.store, &app.google, LoginOptions::oidc("google")).await {
        Ok(redirect) => (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, redirect.url().to_string()),
                (header::SET_COOKIE, redirect.set_cookie(&app.cookie)),
            ],
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<h1>Error: {}</h1>",
                html_escape(&error.to_string())
            )),
        )
            .into_response(),
    }
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

async fn callback(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    auth: AuthCallback,
) -> Response {
    let challenge_id = ChallengeCookie::read(
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok()),
    );
    let page = match finish_login(
        &app.store,
        &app.google,
        "google",
        challenge_id.as_deref(),
        &auth,
    )
    .await
    {
        Ok(completed) => (
            StatusCode::OK,
            Html(format!(
                "<h1>Welcome, {}!</h1>",
                html_escape(&completed.user().name)
            )),
        ),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Html(format!(
                "<h1>Sign-in failed: {}</h1>",
                html_escape(&error.to_string())
            )),
        ),
    };
    ([(header::SET_COOKIE, app.cookie.clear())], page).into_response()
}
