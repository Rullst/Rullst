//! Nexus behind `NexusAuthPolicy::application`: the application's own sign-in
//! decides on every request, and Nexus keeps its CSRF and role boundaries.
//! No database is needed: every request ends before a query runs.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header, request::Parts},
};
use rullst_nexus::{
    FieldKind, FieldMeta, Nexus, NexusAccess, NexusAuthPolicy, NexusAuthorizer,
    NexusAuthorizerError, NexusModel,
};
use tower::ServiceExt;

const CSRF: &str = "nexus_application_csrf_fixture";

/// A session lookup stand-in: `cookie: session=<who>` names the user.
struct SessionAdmins;

impl NexusAuthorizer for SessionAdmins {
    async fn authorize(&self, request: &Parts) -> Result<NexusAccess, NexusAuthorizerError> {
        let session = request
            .headers
            .get_all(header::COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .find_map(|pair| pair.trim().strip_prefix("session="));
        match session {
            None => Ok(NexusAccess::SignIn),
            Some("admin") => Ok(NexusAccess::allow("user:1")),
            Some("broken") => Err(NexusAuthorizerError::new("session store unavailable")),
            Some(_) => Ok(NexusAccess::Deny),
        }
    }
}

struct Note;

impl NexusModel for Note {
    fn nexus_table() -> &'static str {
        "notes"
    }
    fn nexus_label() -> &'static str {
        "Notes"
    }
    fn nexus_fields() -> Vec<FieldMeta> {
        vec![
            FieldMeta::new("id", "ID", FieldKind::Number).readonly(),
            FieldMeta::new("title", "Title", FieldKind::Text),
        ]
    }
}

fn app() -> Router {
    Nexus::new()
        .with_brand("Application Admin")
        .with_auth_policy(NexusAuthPolicy::application(SessionAdmins))
        .register::<Note>()
        .try_build()
        .expect("application policy builds without credentials")
}

async fn send(request: Request<Body>) -> (StatusCode, axum::http::HeaderMap, String) {
    let response = app().oneshot(request).await.expect("router response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("bounded body");
    (status, headers, String::from_utf8_lossy(&body).into_owned())
}

fn get(uri: &str, session: Option<&str>) -> Request<Body> {
    let mut request = Request::get(uri);
    if let Some(session) = session {
        request = request.header(header::COOKIE, format!("session={session}"));
    }
    request.body(Body::empty()).expect("valid request")
}

fn post(uri: &str, session: &str, csrf: bool) -> Request<Body> {
    let cookie = if csrf {
        format!("session={session}; rullst_csrf={CSRF}")
    } else {
        format!("session={session}")
    };
    let mut request = Request::post(uri)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if csrf {
        request = request.header("x-csrf-token", CSRF);
    }
    request.body(Body::from("title=x")).expect("valid request")
}

#[tokio::test]
async fn a_signed_in_administrator_opens_nexus_without_a_second_login() {
    let (status, _, body) = send(get("/", Some("admin"))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Application Admin"), "{body}");
    assert!(body.contains("/nexus/table/notes"), "{body}");
}

#[tokio::test]
async fn signed_out_requests_go_to_the_application_login() {
    let (status, headers, _) = send(get("/table/notes", None)).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers[header::LOCATION], "/login");

    let mut fragment = get("/table/notes/search?q=x", None);
    fragment
        .headers_mut()
        .insert("hx-request", "true".parse().expect("header value"));
    let (status, headers, _) = send(fragment).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(headers["hx-redirect"], "/login");
}

#[tokio::test]
async fn ordinary_users_and_authorizer_failures_never_reach_the_data() {
    for uri in [
        "/",
        "/table/notes",
        "/table/notes/7/edit",
        "/assets/nexus.js",
    ] {
        let (status, _, body) = send(get(uri, Some("learner"))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri}");
        assert_eq!(body, "", "{uri}");
        let (status, _, _) = send(get(uri, Some("broken"))).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{uri}");
    }
    let (status, _, _) = send(post("/table/notes", "learner", true)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn mutations_keep_the_csrf_boundary() {
    let (status, _, body) = send(post("/table/notes", "admin", false)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(body.contains("CSRF"), "{body}");

    // With the token the request passes CSRF and reaches the handler, which
    // finds no such table.
    let (status, _, body) = send(post("/table/missing", "admin", true)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
}
