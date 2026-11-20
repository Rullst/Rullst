//! The per-launch key of the remote Studio.
//!
//! Every account on the machine can connect to `127.0.0.1`, so the loopback
//! boundary alone would let another local user read the monitoring documents
//! Studio fetches with the decrypted tokens, and change its settings. Each
//! start therefore draws a random 256-bit key and prints
//! `http://127.0.0.1:<port>/studio?key=<key>`. Opening that address exchanges
//! the key for an `HttpOnly`, `SameSite=Strict` session cookie and drops it
//! from the address; every other request without the cookie gets `403`.
//! Anyone who can read the printed address (for example in a shared log) can
//! open Studio until it restarts, and browsers do not keep cookies apart by
//! port, so a server on another `127.0.0.1` port that this browser visits
//! from a Studio page can receive the cookie.

use super::router::{RemoteState, form_token_valid};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use std::sync::Arc;

/// Page shown to a request without the key.
const LOCKED_PAGE: &str = "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Rullst Studio</title></head><body><h1>Rullst Studio is locked</h1><p>Open the address that <code>rullst-studio</code> printed when it started; it ends in <code>?key=</code> and changes at every start.</p></body></html>";

/// The key of one Studio process and the port it listens on.
pub(crate) struct LaunchKey {
    /// 64 hexadecimal digits.
    pub(super) key: String,
    port: u16,
}

impl LaunchKey {
    pub(crate) fn new(key: String, port: u16) -> Self {
        Self { key, port }
    }

    /// The address printed at start-up.
    pub(crate) fn address(&self) -> String {
        format!("http://127.0.0.1:{}/studio?key={}", self.port, self.key)
    }

    /// Named after the port, so two Studios on one machine keep their own.
    pub(crate) fn cookie_name(&self) -> String {
        format!("rullst_studio_{}", self.port)
    }

    /// A `Cookie` header carries this process's key.
    fn cookie_matches(&self, headers: &HeaderMap) -> bool {
        let prefix = format!("{}=", self.cookie_name());
        headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .filter_map(|pair| pair.trim().strip_prefix(prefix.as_str()))
            .any(|value| form_token_valid(&self.key, value))
    }

    /// The query string carries this process's key as `key=`.
    fn query_matches(&self, uri: &Uri) -> bool {
        uri.query()
            .into_iter()
            .flat_map(|query| query.split('&'))
            .filter_map(|pair| pair.strip_prefix("key="))
            .any(|value| form_token_valid(&self.key, value))
    }

    /// Sets the session cookie and sends the browser to the Studio home
    /// without the key in its address.
    fn unlock(&self) -> Response {
        let cookie = format!(
            "{}={}; Path=/; HttpOnly; SameSite=Strict",
            self.cookie_name(),
            self.key
        );
        let Ok(cookie) = HeaderValue::from_str(&cookie) else {
            return locked();
        };
        let mut response = Redirect::to("/studio").into_response();
        response.headers_mut().insert(header::SET_COOKIE, cookie);
        response
    }
}

fn locked() -> Response {
    (StatusCode::FORBIDDEN, Html(LOCKED_PAGE)).into_response()
}

/// Lets through a request with the session cookie, and turns a `GET` with
/// the key in its query string into that cookie.
pub(crate) async fn require_launch_key(
    State(state): State<Arc<RemoteState>>,
    request: Request,
    next: Next,
) -> Response {
    let launch = &state.launch;
    if launch.cookie_matches(request.headers()) {
        return next.run(request).await;
    }
    if request.method() == Method::GET && launch.query_matches(request.uri()) {
        return launch.unlock();
    }
    locked()
}
