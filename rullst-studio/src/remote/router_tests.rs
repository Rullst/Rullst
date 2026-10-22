//! The remote Studio's pages, forms and boundary (`TM-STUDIO-3`,
//! `REMOTE-01`, `REMOTE-02`, `REMOTE-05`, `REMOTE-07`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::client::FetchLimits;
use super::router::{RemoteState, form_token_valid, router};
use super::server_test::{TOKEN, TempDir, start};
use super::{RemoteStudioOptions, state};
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:5556";

struct Studio {
    app: Router,
    state: Arc<RemoteState>,
    _directory: TempDir,
}

fn studio() -> Studio {
    let directory = TempDir::new("router");
    let mut options = RemoteStudioOptions::new(directory.paths());
    options.limits = FetchLimits {
        max_bytes: 1024 * 1024,
        timeout: Duration::from_millis(800),
        connect_timeout: Duration::from_millis(500),
    };
    let state = Arc::new(state(&options).unwrap());
    Studio {
        app: router(state.clone()),
        state,
        _directory: directory,
    }
}

fn request(
    method: Method,
    uri: &str,
    peer: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, HOST);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::from(body.to_string())).unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        peer.parse::<std::net::SocketAddr>().unwrap(),
    ));
    request
}

async fn get(studio: &Studio, uri: &str) -> (StatusCode, String) {
    let response = studio
        .app
        .clone()
        .oneshot(request(Method::GET, uri, "127.0.0.1:40000", &[], ""))
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn post(studio: &Studio, uri: &str, origin: Option<&str>, body: &str) -> StatusCode {
    let mut headers = vec![("content-type", "application/x-www-form-urlencoded")];
    if let Some(origin) = origin {
        headers.push(("origin", origin));
    }
    studio
        .app
        .clone()
        .oneshot(request(
            Method::POST,
            uri,
            "127.0.0.1:40000",
            &headers,
            body,
        ))
        .await
        .unwrap()
        .status()
}

fn add_form(studio: &Studio, name: &str, base_url: &str) -> String {
    format!(
        "form_token={}&name={}&base_url={}&token={TOKEN}",
        studio.state.form_token,
        urlencoding::encode(name),
        urlencoding::encode(base_url)
    )
}

const ORIGIN: &str = "http://127.0.0.1:5556";

#[tokio::test]
async fn projects_are_added_and_removed_through_the_protected_forms() {
    let studio = studio();
    let (status, page) = get(&studio, "/studio").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("No projects yet"));

    let form = add_form(&studio, "shop", "https://shop.example");
    assert_eq!(
        post(&studio, "/studio/projects", Some(ORIGIN), &form).await,
        StatusCode::SEE_OTHER
    );
    let (_, page) = get(&studio, "/studio?notice=added").await;
    assert!(page.contains("Project added"));
    assert!(page.contains("https://shop.example/"));
    assert!(page.contains("token stored"));
    assert!(!page.contains(TOKEN));

    let remove = format!("form_token={}&name=shop", studio.state.form_token);
    assert_eq!(
        post(&studio, "/studio/projects/remove", Some(ORIGIN), &remove).await,
        StatusCode::SEE_OTHER
    );
    assert!(studio.state.store.load().unwrap().is_empty());
}

#[tokio::test]
async fn cross_origin_or_tokenless_forms_change_nothing() {
    let studio = studio();
    let form = add_form(&studio, "shop", "https://shop.example");
    assert_eq!(
        post(&studio, "/studio/projects", None, &form).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        post(
            &studio,
            "/studio/projects",
            Some("https://attacker.example"),
            &form
        )
        .await,
        StatusCode::FORBIDDEN
    );
    let wrong_token = form.replacen(&studio.state.form_token, &"0".repeat(64), 1);
    assert_eq!(
        post(&studio, "/studio/projects", Some(ORIGIN), &wrong_token).await,
        StatusCode::FORBIDDEN
    );
    let missing_token =
        format!("form_token=&name=shop&base_url=https%3A%2F%2Fshop.example&token={TOKEN}");
    assert_eq!(
        post(&studio, "/studio/projects", Some(ORIGIN), &missing_token).await,
        StatusCode::FORBIDDEN
    );
    assert!(studio.state.store.load().unwrap().is_empty());

    let invalid = add_form(&studio, "shop", "http://shop.example");
    assert_eq!(
        post(&studio, "/studio/projects", Some(ORIGIN), &invalid).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(studio.state.store.load().unwrap().is_empty());
}

#[tokio::test]
async fn only_direct_loopback_requests_reach_studio() {
    let studio = studio();
    let remote = studio
        .app
        .clone()
        .oneshot(request(Method::GET, "/studio", "192.0.2.10:40000", &[], ""))
        .await
        .unwrap();
    assert_eq!(remote.status(), StatusCode::FORBIDDEN);
    let mut public_host = request(Method::GET, "/studio", "127.0.0.1:40000", &[], "");
    public_host
        .headers_mut()
        .insert(header::HOST, "studio.example".parse().unwrap());
    assert_eq!(
        studio
            .app
            .clone()
            .oneshot(public_host)
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn local_only_pages_do_not_exist_in_remote_mode() {
    let studio = studio();
    for path in [
        "/studio/db",
        "/studio/env",
        "/studio/ai",
        "/studio/api",
        "/studio/migrations",
        "/studio/features",
        "/studio/jobs",
        "/studio/cache",
        "/studio/requests",
    ] {
        assert_eq!(get(&studio, path).await.0, StatusCode::NOT_FOUND, "{path}");
    }
    assert_eq!(
        get(&studio, "/studio/assets/studio.css").await.0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn the_switcher_selects_projects_and_pages_render_remote_values_escaped() {
    let (base, _) = start(None).await;
    let studio = studio();
    studio
        .state
        .store
        .add("orders", &format!("{base}ok/"), TOKEN)
        .unwrap();
    studio
        .state
        .store
        .add("hostile", &format!("{base}xss/"), TOKEN)
        .unwrap();
    studio
        .state
        .store
        .add("down", &format!("{base}slow/"), TOKEN)
        .unwrap();

    // Without a project, the first one is shown.
    let (status, page) = get(&studio, "/studio/radar").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("Overview · orders"));
    assert!(page.contains(r#"<option value="orders" selected>orders</option>"#));
    assert!(page.contains(r#"<option value="hostile">hostile</option>"#));
    assert!(page.contains("0.333"));

    for path in ["/studio/routes", "/studio/queues", "/studio/queries"] {
        let (status, page) = get(&studio, &format!("{path}?project=orders")).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(page.contains("orders"), "{path}");
    }
    let (_, routes) = get(&studio, "/studio/routes?project=orders").await;
    assert!(routes.contains("/orders/{id}"));
    let (_, queries) = get(&studio, "/studio/queries?project=orders").await;
    assert!(queries.contains("LineItem.find (line_items)"));

    for path in ["/studio/radar", "/studio/routes", "/studio/queries"] {
        let (status, page) = get(&studio, &format!("{path}?project=hostile")).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(!page.contains("<script>alert"), "{path}");
        assert!(!page.contains("<img src=x"), "{path}");
        assert!(page.contains("&lt;script&gt;"), "{path}");
    }

    let (status, page) = get(&studio, "/studio/radar?project=down").await;
    assert_eq!(status, StatusCode::OK);
    assert!(page.contains("down is unavailable"));
    assert!(page.contains("no answer within the time limit"));
    let (_, projects) = get(&studio, "/studio").await;
    assert!(projects.contains("Last read failed"));

    let (status, page) = get(&studio, "/studio/radar?project=%3Cscript%3E").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(page.contains("Unknown project"));
    assert!(!page.contains("<script>"));
}

#[test]
fn form_tokens_compare_whole_values() {
    assert!(form_token_valid("abc123", "abc123"));
    assert!(!form_token_valid("abc123", "abc124"));
    assert!(!form_token_valid("abc123", "abc12"));
    assert!(!form_token_valid("abc123", ""));
}
