//! The exact response headers of the server stack, in order, for every
//! request kind the `Vary: HX-Request` step and the security baseline treat
//! differently. The access log on and off must give the same headers.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::Router;
use crate::server::console::Recording;
use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode, header};
use tower::ServiceExt;

const COOKIE: &str = "rullst_csrf=AbCdEfGhIjKlMnOpQrStUvWxYz012345";

fn application() -> Router {
    async fn page(htmx: crate::HtmxRequest) -> axum::response::Html<String> {
        crate::render_page(&htmx, "T", crate::html! { <p>"x"</p> })
    }
    Router::new()
        .route("/page", axum::routing::get(page))
        .route("/text", axum::routing::get(|| async { "Hello World" }))
        .route(
            "/own-vary",
            axum::routing::get(|| async {
                (
                    [(header::VARY, HeaderValue::from_static("Accept-Encoding"))],
                    axum::response::Html("<p>own</p>"),
                )
            }),
        )
        .route(
            "/write",
            axum::routing::post(|| async { StatusCode::NO_CONTENT }),
        )
}

fn stack(environment: Environment, recording: Recording) -> axum::Router {
    stack_of(Server::new(application()), environment, recording)
}

fn stack_of(mut server: Server, environment: Environment, recording: Recording) -> axum::Router {
    server.access_log_recording = recording;
    server
        .into_static_app(SecurityConfig::default(), environment)
        .unwrap()
}

/// The requests of one transcript: a label and the request.
fn requests() -> Vec<(&'static str, Request<Body>)> {
    let get = |uri: &str| Request::get(uri).header(header::USER_AGENT, "Mozilla/5.0");
    vec![
        ("page", get("/page").body(Body::empty()).unwrap()),
        (
            "htmx page",
            get("/page")
                .header("HX-Request", "true")
                .body(Body::empty())
                .unwrap(),
        ),
        (
            "page with cookie",
            get("/page")
                .header(header::COOKIE, COOKIE)
                .body(Body::empty())
                .unwrap(),
        ),
        ("text", get("/text").body(Body::empty()).unwrap()),
        ("own vary", get("/own-vary").body(Body::empty()).unwrap()),
        ("missing", get("/missing").body(Body::empty()).unwrap()),
        (
            "write without token",
            Request::post("/write")
                .header(header::USER_AGENT, "Mozilla/5.0")
                .body(Body::empty())
                .unwrap(),
        ),
        (
            "crawler",
            Request::get("/page")
                .header(header::USER_AGENT, "GPTBot/1.0")
                .body(Body::empty())
                .unwrap(),
        ),
        (
            "https page",
            get("https://app.example/page").body(Body::empty()).unwrap(),
        ),
    ]
}

/// `value` with the CSRF token of a `rullst_csrf` cookie and a CSP nonce
/// replaced by placeholders.
fn normalized(value: &str) -> String {
    let mut text = value.to_string();
    if let Some(rest) = text.strip_prefix("rullst_csrf=") {
        let end = rest.find(';').unwrap_or(rest.len());
        text = format!("rullst_csrf=<token>{}", &rest[end..]);
    }
    let mut from = 0;
    while let Some(found) = text[from..].find("'nonce-") {
        let start = from + found + 7;
        let end = start + text[start..].find('\'').unwrap();
        text.replace_range(start..end, "<nonce>");
        from = start;
    }
    text
}

/// Status and headers, in order, of each request of [`requests`].
async fn transcript(app: &axum::Router) -> String {
    transcript_of(app, requests()).await
}

async fn transcript_of(app: &axum::Router, requests: Vec<(&str, Request<Body>)>) -> String {
    let mut lines = Vec::new();
    for (label, request) in requests {
        let response = app.clone().oneshot(request).await.unwrap();
        lines.push(format!("{label}: {}", response.status().as_u16()));
        for (name, value) in response.headers() {
            lines.push(format!("  {name}: {}", normalized(value.to_str().unwrap())));
        }
    }
    lines.join("\n")
}

async fn assert_transcript(environment: Environment, expected: &str) {
    for recording in [Recording::Off, Recording::All] {
        let actual = transcript(&stack(environment, recording)).await;
        assert_eq!(actual, expected, "{environment:?}, {recording:?}");
    }
}

/// `expected` with its placeholders replaced: `{FIXED}` is the fixed
/// headers, `{BASELINE}` `Cache-Control` and them, `{HSTS}` the default
/// `Strict-Transport-Security`, `{PAGE}` and `{OTHER}` the referrer policy,
/// COEP and the CSP of a response that may be a document and of one that
/// cannot.
fn expand(expected: &str) -> String {
    const FIXED: &str = "  x-frame-options: DENY
  x-content-type-options: nosniff
  x-xss-protection: 0
  permissions-policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()
  cross-origin-opener-policy: same-origin
  cross-origin-resource-policy: same-origin";
    const HSTS: &str = "  strict-transport-security: max-age=31536000; includeSubDomains";
    const POLICIES: &str = "  referrer-policy: strict-origin-when-cross-origin
  cross-origin-embedder-policy: require-corp";
    const PAGE: &str = "  content-security-policy: default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'nonce-<nonce>'; style-src 'self' 'nonce-<nonce>'; img-src 'self' data:; connect-src 'self'; font-src 'self'; worker-src 'self' blob:";
    const OTHER: &str = "  content-security-policy: default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; font-src 'self'; worker-src 'self' blob:";
    expected
        .trim()
        .replace("{BASELINE}", &format!("  cache-control: no-store\n{FIXED}"))
        .replace("{FIXED}", FIXED)
        .replace("{HSTS}", HSTS)
        .replace("{PAGE}", &format!("{POLICIES}\n{PAGE}"))
        .replace("{OTHER}", &format!("{POLICIES}\n{OTHER}"))
}

/// Test and development: no `Secure` cookie attribute and no HSTS over plain
/// HTTP, and the WAF lets the crawler through.
const DEVELOPMENT: &str = "
page: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax
{BASELINE}
{PAGE}
  content-length: 734
htmx page: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax
{BASELINE}
{PAGE}
  content-length: 8
page with cookie: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
{BASELINE}
{PAGE}
  content-length: 734
text: 200
  content-type: text/plain; charset=utf-8
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax
{BASELINE}
{OTHER}
  content-length: 11
own vary: 200
  content-type: text/html; charset=utf-8
  vary: Accept-Encoding
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax
{BASELINE}
{PAGE}
  content-length: 10
missing: 404
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax
{BASELINE}
{PAGE}
write without token: 403
  content-type: text/plain; charset=utf-8
{BASELINE}
{OTHER}
  content-length: 25
crawler: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax
{BASELINE}
{PAGE}
  content-length: 734
https page: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{PAGE}
  content-length: 734
";

/// Production: a `Secure` cookie and HSTS on every response, and the WAF
/// refuses the crawler before CSRF and the `Vary` step.
const PRODUCTION: &str = "
page: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{PAGE}
  content-length: 734
htmx page: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{PAGE}
  content-length: 8
page with cookie: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
{BASELINE}
{HSTS}
{PAGE}
  content-length: 734
text: 200
  content-type: text/plain; charset=utf-8
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{OTHER}
  content-length: 11
own vary: 200
  content-type: text/html; charset=utf-8
  vary: Accept-Encoding
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{PAGE}
  content-length: 10
missing: 404
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{PAGE}
write without token: 403
  content-type: text/plain; charset=utf-8
{BASELINE}
{HSTS}
{OTHER}
  content-length: 25
crawler: 403
  content-type: text/plain; charset=utf-8
{BASELINE}
{HSTS}
{OTHER}
  content-length: 66
https page: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{PAGE}
  content-length: 734
";

#[tokio::test]
async fn test_environment_headers_are_pinned() {
    assert_transcript(Environment::Test, &expand(DEVELOPMENT)).await;
}

#[tokio::test]
async fn development_headers_are_pinned() {
    assert_transcript(Environment::Development, &expand(DEVELOPMENT)).await;
}

#[tokio::test]
async fn staging_and_production_headers_are_pinned() {
    for environment in [Environment::Staging, Environment::Production] {
        assert_transcript(environment, &expand(PRODUCTION)).await;
    }
}

#[tokio::test]
async fn layers_between_the_baseline_and_the_routes_keep_the_headers() {
    use crate::lifecycle::ApplicationLifecycle;
    use crate::resilience::{RateLimitConfig, RateLimiter, TrafficShield, TrafficShieldConfig};
    let lifecycle = ApplicationLifecycle::new();
    lifecycle.mark_ready().unwrap();
    let shield = TrafficShield::new(TrafficShieldConfig::new().with_db_probe(false));
    let limiter = RateLimiter::new(RateLimitConfig::new(1000.0, 1000.0));
    let masking = SecurityConfig {
        enable_pii_masking: true,
        ..SecurityConfig::default()
    };
    let configurations = [
        (
            "lifecycle",
            Server::new(application()).with_lifecycle(lifecycle),
            None,
        ),
        ("shield", Server::new(application()).shield(shield), None),
        (
            "rate limit",
            Server::new(application()).rate_limit(limiter),
            None,
        ),
        ("PII masking", Server::new(application()), Some(masking)),
    ];
    for (label, mut server, security) in configurations {
        server.access_log_recording = Recording::Off;
        let app = server
            .into_static_app(security.unwrap_or_default(), Environment::Production)
            .unwrap();
        assert_eq!(transcript(&app).await, expand(PRODUCTION), "{label}");
    }
}

/// Lifecycle admission, Traffic Shield and the rate limiter refuse with plain
/// text, before the routes and the `Vary` step.
const REFUSALS: &str = "
draining: 503
  content-type: text/plain; charset=utf-8
  cache-control: no-store
  retry-after: 1
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{FIXED}
{HSTS}
{OTHER}
  content-length: 52
shed: 503
  retry-after: 5
  content-type: text/plain; charset=utf-8
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{OTHER}
  content-length: 53
limited: 200
  content-type: text/html; charset=utf-8
  vary: HX-Request
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{PAGE}
  content-length: 734
limited: 429
  content-type: text/plain; charset=utf-8
  set-cookie: rullst_csrf=<token>; Path=/; SameSite=Lax; Secure
{BASELINE}
{HSTS}
{OTHER}
  content-length: 44
";

#[tokio::test]
async fn refusals_of_the_layers_inside_the_baseline_are_pinned() {
    use crate::lifecycle::ApplicationLifecycle;
    use crate::resilience::{RateLimitConfig, RateLimiter, TrafficShield, TrafficShieldConfig};
    let shield = TrafficShieldConfig::new()
        .with_db_probe(false)
        .with_max_active_requests(0);
    let limiter = RateLimiter::new(RateLimitConfig::new(1.0, 0.001));
    let configurations = [
        (
            "draining",
            Server::new(application()).with_lifecycle(ApplicationLifecycle::new()),
        ),
        (
            "shed",
            Server::new(application()).shield(TrafficShield::new(shield)),
        ),
        ("limited", Server::new(application()).rate_limit(limiter)),
    ];
    let mut lines = Vec::new();
    for (label, server) in configurations {
        let app = stack_of(server, Environment::Production, Recording::Off);
        let requests = (0..1 + usize::from(label == "limited"))
            .map(|_| (label, Request::get("/page").body(Body::empty()).unwrap()))
            .collect();
        lines.push(transcript_of(&app, requests).await);
    }
    assert_eq!(lines.join("\n"), expand(REFUSALS));
}

#[path = "stack_vary_tests.rs"]
mod vary;
