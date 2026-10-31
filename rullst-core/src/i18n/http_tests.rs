use super::*;
use axum::Router;
use axum::routing::get;
use tower::ServiceExt;

static LOCALES: Locales = Locales::__from_catalog(&["en", "pt-BR"], 0);

fn app(environment: Option<Environment>) -> Router {
    let router = Router::new()
        .route("/", get(|locale: Locale| async move { locale.tag() }))
        .route("/locale", LOCALES.switch_route())
        .layer(LOCALES.layer());
    match environment {
        Some(environment) => router.layer(axum::Extension(environment)),
        None => router,
    }
}

async fn send(
    router: Router,
    request: axum::http::Request<Body>,
) -> (StatusCode, HeaderMap, String) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 1 << 16)
        .await
        .unwrap();
    (status, headers, String::from_utf8(body.to_vec()).unwrap())
}

fn switch() -> axum::http::request::Builder {
    axum::http::Request::post("/locale")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
}

fn post(body: &str) -> axum::http::Request<Body> {
    switch().body(Body::from(body.to_owned())).unwrap()
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

#[tokio::test]
async fn the_layer_negotiates_the_locale_for_the_extractor() {
    let request = axum::http::Request::get("/")
        .header(header::ACCEPT_LANGUAGE, "pt-PT, en;q=0.5")
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(app(None), request).await.2, "pt-BR");
    let request = axum::http::Request::get("/")
        .header(header::COOKIE, "rullst_locale=en")
        .header(header::ACCEPT_LANGUAGE, "pt-BR")
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(app(None), request).await.2, "en");
}

#[tokio::test]
async fn the_extractor_names_a_missing_layer() {
    let router = Router::new().route("/", get(|locale: Locale| async move { locale.tag() }));
    let (status, _, body) = send(
        router,
        axum::http::Request::get("/").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        body,
        "the locale layer is not installed: add `.layer(LOCALES.layer())` to the router"
    );
}

#[tokio::test]
async fn switching_sets_the_cookie_and_redirects_to_a_local_path() {
    let (status, headers, _) = send(
        app(Some(Environment::Development)),
        post("locale=PT-br&next=%2Flessons%3Fpage%3D2"),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(header_value(&headers, "location"), Some("/lessons?page=2"));
    assert_eq!(
        header_value(&headers, "set-cookie"),
        Some("rullst_locale=pt-BR; Path=/; Max-Age=31536000; SameSite=Lax; HttpOnly")
    );
    assert_eq!(headers.get("hx-redirect"), None);

    let (_, headers, _) = send(app(Some(Environment::Production)), post("locale=en")).await;
    assert_eq!(header_value(&headers, "location"), Some("/"));
    assert_eq!(
        header_value(&headers, "set-cookie"),
        Some("rullst_locale=en; Path=/; Max-Age=31536000; SameSite=Lax; HttpOnly; Secure")
    );
}

#[tokio::test]
async fn htmx_switches_reload_the_whole_page() {
    let request = switch()
        .header("HX-Request", "true")
        .body(Body::from("locale=en&next=/home"))
        .unwrap();
    let (status, headers, _) = send(app(Some(Environment::Development)), request).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(header_value(&headers, "hx-redirect"), Some("/home"));
    assert_eq!(headers.get("location"), None);
    assert!(
        header_value(&headers, "set-cookie").is_some_and(|c| c.starts_with("rullst_locale=en;"))
    );

    let request = switch()
        .header("HX-Request", "false")
        .body(Body::from("locale=en"))
        .unwrap();
    let (status, _, _) = send(app(Some(Environment::Development)), request).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn switching_rejects_unsupported_malformed_and_oversized_forms() {
    let (status, headers, body) = send(app(None), post("locale=fr&next=/")).await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::BAD_REQUEST, "Unsupported locale")
    );
    assert_eq!(headers.get("set-cookie"), None);
    let (status, _, body) = send(app(None), post("next=/")).await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::BAD_REQUEST, "Invalid locale form")
    );
    let at_limit = format!("locale=en&x={}", "a".repeat(4096 - 12));
    assert_eq!(at_limit.len(), 4096);
    assert_eq!(
        send(app(None), post(&at_limit)).await.0,
        StatusCode::SEE_OTHER
    );
    let over = format!("{at_limit}a");
    let (status, _, body) = send(app(None), post(&over)).await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::PAYLOAD_TOO_LARGE, "Locale form too large")
    );
    let get = axum::http::Request::get("/locale")
        .body(Body::empty())
        .unwrap();
    assert_eq!(send(app(None), get).await.0, StatusCode::METHOD_NOT_ALLOWED);
}

#[test]
fn only_same_origin_paths_are_redirect_targets() {
    for local in [
        "/",
        "/a/b?c=d#e",
        "/%2F%2Fevil.com",
        &format!("/{}", "a".repeat(2047)),
    ] {
        assert!(is_local_path(local), "{local}");
    }
    for remote in [
        "",
        "evil.com",
        "https://evil.com",
        "//evil.com",
        "/\\evil.com",
        "/a\\b",
        "/\t/evil.com",
        "/a b",
        "/caf\u{e9}",
        "/\u{7f}",
        &format!("/{}", "a".repeat(2048)),
    ] {
        assert!(!is_local_path(remote), "{remote:?}");
    }
}

#[tokio::test]
async fn unsafe_next_targets_fall_back_to_the_root() {
    let (_, headers, _) = send(app(None), post("locale=en&next=%2F%2Fevil.com")).await;
    assert_eq!(header_value(&headers, "location"), Some("/"));
}

#[test]
fn secure_cookies_follow_the_environment() {
    let with = |environment| {
        let mut extensions = Extensions::new();
        extensions.insert(environment);
        secure_cookies(&extensions)
    };
    assert!(!with(Environment::Development));
    assert!(with(Environment::Staging));
    assert!(with(Environment::Production));
    // Both reads must see the same selectors, which sibling tests change.
    let _lock = crate::server::TEST_ENV_LOCK.blocking_lock();
    let detected = Environment::detect(None).map_or(true, Environment::requires_secure_defaults);
    assert_eq!(secure_cookies(&Extensions::new()), detected);
}

#[test]
fn the_rejection_is_a_server_error() {
    let response = MissingLocale.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

/// A service that is never ready, to show the layer forwards readiness.
#[derive(Clone)]
struct NeverReady;

impl tower_service::Service<axum::http::Request<Body>> for NeverReady {
    type Response = Response;
    type Error = &'static str;
    type Future = std::future::Ready<Result<Response, &'static str>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Err("inner not ready"))
    }

    fn call(&mut self, _request: axum::http::Request<Body>) -> Self::Future {
        std::future::ready(Err("called"))
    }
}

#[tokio::test]
async fn the_service_forwards_readiness_and_inserts_the_locale() {
    use tower_layer::Layer;
    let mut service = LOCALES.layer().layer(NeverReady);
    let ready =
        std::future::poll_fn(|cx| tower_service::Service::poll_ready(&mut service, cx)).await;
    assert_eq!(ready, Err("inner not ready"));

    let echo = tower::service_fn(|request: axum::http::Request<Body>| async move {
        let tag = request
            .extensions()
            .get::<Locale>()
            .map(|locale| locale.tag());
        Ok::<_, std::convert::Infallible>(tag)
    });
    let request = axum::http::Request::get("/")
        .header(header::ACCEPT_LANGUAGE, "pt")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        LOCALES.layer().layer(echo).oneshot(request).await,
        Ok(Some("pt-BR"))
    );
}
