use super::*;
use crate::nexus::NexusAuthPolicy;
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use tower::ServiceExt;

/// Answers from the `x-test-access` request header, standing in for an
/// application session lookup.
struct HeaderAuthorizer;

impl NexusAuthorizer for HeaderAuthorizer {
    async fn authorize(&self, request: &Parts) -> Result<NexusAccess, NexusAuthorizerError> {
        let value = request
            .headers
            .get("x-test-access")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        match value.split_once('=') {
            Some(("allow", actor)) => Ok(NexusAccess::allow(actor)),
            Some(("error", reason)) => Err(NexusAuthorizerError::new(reason)),
            _ if value == "deny" => Ok(NexusAccess::Deny),
            _ => Ok(NexusAccess::SignIn),
        }
    }
}

/// Echoes the actor ID of the principal the policy inserted.
async fn actor(Extension(principal): Extension<NexusPrincipal>) -> String {
    principal.actor_id().to_owned()
}

fn protected(access: NexusApplicationAuth) -> Router {
    NexusAuthPolicy::Application(access)
        .protect_router(
            Router::new()
                .route("/", get(actor))
                .route("/save", post(actor)),
        )
        .expect("valid application policy")
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, axum::http::HeaderMap, String) {
    let response = app.clone().oneshot(request).await.expect("router response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .expect("bounded body");
    (status, headers, String::from_utf8_lossy(&body).into_owned())
}

fn request(method: &str, uri: &str, access: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(access) = access {
        builder = builder.header("x-test-access", access);
    }
    builder.body(Body::empty()).expect("valid request")
}

#[tokio::test]
async fn an_allowed_administrator_reaches_nexus_as_the_returned_actor() {
    let app = protected(NexusApplicationAuth::new(HeaderAuthorizer));
    let (status, _, body) = send(&app, request("GET", "/", Some("allow=user:42"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "user:42");

    let (status, _, body) = send(&app, request("POST", "/save", Some("allow=user:7"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "user:7");
}

#[tokio::test]
async fn a_signed_out_page_request_is_redirected_to_the_login_page() {
    let app = protected(NexusApplicationAuth::new(HeaderAuthorizer));
    for method in ["GET", "HEAD"] {
        let (status, headers, _) = send(&app, request(method, "/", None)).await;
        assert_eq!(status, StatusCode::SEE_OTHER, "{method}");
        assert_eq!(headers[header::LOCATION], "/login", "{method}");
        assert!(!headers.contains_key("hx-redirect"), "{method}");
    }

    let custom = protected(
        NexusApplicationAuth::new(HeaderAuthorizer).with_login_path("/account/sign-in?to=nexus"),
    );
    let (status, headers, _) = send(&custom, request("GET", "/", Some("signed-out"))).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers[header::LOCATION], "/account/sign-in?to=nexus");
}

#[tokio::test]
async fn a_signed_out_fragment_or_mutation_gets_401_with_hx_redirect() {
    let app = protected(NexusApplicationAuth::new(HeaderAuthorizer).with_login_path("/signin"));

    let mut fragment = request("GET", "/", None);
    fragment
        .headers_mut()
        .insert("hx-request", HeaderValue::from_static("true"));
    let (status, headers, _) = send(&app, fragment).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(headers["hx-redirect"], "/signin");
    assert!(!headers.contains_key(header::LOCATION));
    // No Basic challenge: the browser must not open a credential prompt.
    assert!(!headers.contains_key(header::WWW_AUTHENTICATE));

    let (status, headers, body) = send(&app, request("POST", "/save", None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(headers["hx-redirect"], "/signin");
    assert_eq!(body, "");
}

#[tokio::test]
async fn a_user_without_the_permission_is_forbidden() {
    let app = protected(NexusApplicationAuth::new(HeaderAuthorizer));
    for (method, uri) in [("GET", "/"), ("POST", "/save")] {
        let (status, headers, body) = send(&app, request(method, uri, Some("deny"))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method}");
        assert!(!headers.contains_key(header::LOCATION));
        assert_eq!(body, "");
    }
}

#[tokio::test]
async fn an_authorizer_error_denies_the_request() {
    let app = protected(NexusApplicationAuth::new(HeaderAuthorizer));
    for (method, uri) in [("GET", "/"), ("POST", "/save")] {
        let (status, _, body) = send(&app, request(method, uri, Some("error=database down"))).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{method}");
        assert_eq!(body, "");
    }
}

#[tokio::test]
async fn an_unauditable_actor_id_denies_the_request() {
    let app = protected(NexusApplicationAuth::new(HeaderAuthorizer));
    let oversized = format!("allow={}", "a".repeat(256));
    for access in [
        "allow=",
        "allow= user:1",
        "allow=user:1 ",
        oversized.as_str(),
    ] {
        let (status, _, _) = send(&app, request("GET", "/", Some(access))).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{access:?}");
    }
    let longest = format!("allow={}", "a".repeat(255));
    let (status, _, body) = send(&app, request("GET", "/", Some(&longest))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.len(), 255);
}

#[test]
fn actor_ids_are_bounded_unpadded_text_without_controls() {
    assert!(valid_actor_id("user:42"));
    assert!(valid_actor_id(&"a".repeat(255)));
    for invalid in ["", " a", "a ", "a\tb", "a\nb", "\u{7f}"] {
        assert!(!valid_actor_id(invalid), "{invalid:?}");
    }
    assert!(!valid_actor_id(&"a".repeat(256)));
}

#[test]
fn login_paths_are_local_and_header_safe() {
    for valid in ["/login", "/", "/account/sign-in?return=nexus&x=1"] {
        assert_eq!(validate_login_path(valid), Ok(()), "{valid:?}");
    }
    let longest = format!("/{}", "a".repeat(511));
    assert_eq!(validate_login_path(&longest), Ok(()));
    let oversized = format!("/{}", "a".repeat(512));
    for invalid in [
        "",
        "login",
        "https://evil.example/login",
        "//evil.example/login",
        "/\\evil.example",
        "/log in",
        "/login\r\nx: y",
        "/caf\u{e9}",
        oversized.as_str(),
    ] {
        assert_eq!(
            validate_login_path(invalid),
            Err(NexusBuildError::InvalidLoginPath),
            "{invalid:?}"
        );
    }
}

#[test]
fn an_invalid_login_path_fails_the_build() {
    let policy = NexusAuthPolicy::Application(
        NexusApplicationAuth::new(HeaderAuthorizer).with_login_path("//evil.example"),
    );
    assert!(matches!(
        policy.protect_router::<()>(Router::new()),
        Err(NexusBuildError::InvalidLoginPath)
    ));
    let nexus = crate::Nexus::new()
        .with_auth_policy(NexusAuthPolicy::Application(
            NexusApplicationAuth::new(HeaderAuthorizer).with_login_path("login"),
        ))
        .try_build();
    assert!(matches!(nexus, Err(NexusBuildError::InvalidLoginPath)));
}

#[test]
fn application_ergonomic_policy_never_selects_loopback_in_release_builds() {
    let policy = NexusAuthPolicy::local_development_or_application(HeaderAuthorizer);
    if cfg!(debug_assertions) {
        assert!(matches!(policy, NexusAuthPolicy::LoopbackOnly(_)));
    } else {
        assert!(matches!(policy, NexusAuthPolicy::Application(_)));
    }
    assert!(matches!(
        NexusAuthPolicy::application(HeaderAuthorizer),
        NexusAuthPolicy::Application(access) if access.login_path() == "/login"
    ));
}

#[test]
fn debug_output_names_neither_actor_nor_authorizer() {
    let access = NexusAccess::allow("user:secret-name");
    let output = format!("{access:?}");
    assert_eq!(output, "Allow { actor_id_bytes: 16 }");
    assert_eq!(format!("{:?}", NexusAccess::SignIn), "SignIn");
    assert_eq!(format!("{:?}", NexusAccess::Deny), "Deny");
    let auth = NexusApplicationAuth::new(HeaderAuthorizer).with_login_path("/signin");
    assert_eq!(
        format!("{auth:?}"),
        "NexusApplicationAuth { login_path: \"/signin\", .. }"
    );
}

#[test]
fn authorizer_errors_keep_and_display_their_reason() {
    let error = NexusAuthorizerError::new("session store unavailable");
    assert_eq!(error.reason(), "session store unavailable");
    assert_eq!(
        error.to_string(),
        "Nexus authorizer failed: session store unavailable"
    );
    assert_eq!(
        NexusBuildError::InvalidLoginPath.to_string(),
        "the Nexus login path must be a local path starting with a single '/', of at most 512 visible ASCII characters without '\\'"
    );
}
