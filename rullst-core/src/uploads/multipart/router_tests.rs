//! The extractor behind the real security baseline: CSRF, WAF and limits.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::read_tests::{CONTENT_TYPE, field, file_part, form, token_form, within};
use super::*;
use crate::config::{Environment, SecurityConfig};
use axum::{
    Router,
    http::{Request as HttpRequest, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use tower::ServiceExt;

async fn upload(multipart: Multipart) -> Result<String, MultipartError> {
    let policy = MultipartPolicy::default()
        .max_file_bytes(200_000)
        .allowed_kinds([UploadKind::Pdf, UploadKind::PlainText]);
    let form = multipart.read(&policy).await?;
    let files = form
        .files()
        .iter()
        .map(|file| format!("{}:{:?}:{}", file.field_name(), file.kind(), file.len()))
        .collect::<Vec<_>>();
    Ok(format!(
        "{}|{}",
        form.field("title").unwrap_or_default(),
        files.join(",")
    ))
}

fn app(environment: Environment, exempt: bool) -> Router {
    let mut security = SecurityConfig::default();
    if exempt {
        security.waf_body_exempt_paths = vec!["/upload".to_string()];
    }
    let router = Router::new()
        .route("/", get(|| async { "form" }))
        .route("/upload", post(upload));
    crate::security::apply_security_baseline(router, security, environment).unwrap()
}

/// The CSRF cookie token minted by a first `GET /`.
async fn csrf_token(app: &Router) -> String {
    let response = app
        .clone()
        .oneshot(HttpRequest::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    cookie
        .strip_prefix("rullst_csrf=")
        .and_then(|rest| rest.split(';').next())
        .unwrap()
        .to_owned()
}

async fn post_form(app: &Router, token: Option<&str>, body: Vec<u8>) -> (StatusCode, String) {
    let mut builder = HttpRequest::post("/upload").header(header::CONTENT_TYPE, CONTENT_TYPE);
    if let Some(token) = token {
        builder = builder.header(header::COOKIE, format!("rullst_csrf={token}"));
    }
    let response = within(app.clone().oneshot(builder.body(Body::from(body)).unwrap()))
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn production_reads_a_token_first_upload_behind_csrf_and_waf() {
    let app = app(Environment::Production, false);
    let token = csrf_token(&app).await;
    let body = form(&[
        field("_token", &token),
        field("title", "Plano de aula"),
        file_part("document", "plano.pdf", b"%PDF-1.7\n%%EOF"),
    ]);
    assert_eq!(
        post_form(&app, Some(&token), body).await,
        (
            StatusCode::OK,
            "Plano de aula|document:Some(Pdf):14".to_string()
        )
    );
}

#[tokio::test]
async fn csrf_names_the_ordering_rule_in_every_environment() {
    let production = app(Environment::Production, false);
    let development = app(Environment::Development, false);
    let token = csrf_token(&production).await;
    let with_token = |filler: usize| {
        let mut body = token_form(filler, true);
        let at = body.windows(3).position(|window| window == b"abc").unwrap();
        body.splice(at..at + 3, token.bytes());
        body
    };
    let base = with_token(0).len();
    // The middleware and the extractor share the same boundary: the token
    // part must end within the first 64 KiB.
    let readable = with_token(65_540 - base);
    let late = with_token(65_541 - base);
    for app in [&production, &development] {
        let (status, _) = post_form(app, Some(&token), readable.clone()).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            post_form(app, Some(&token), late.clone()).await,
            (
                StatusCode::FORBIDDEN,
                crate::security::MULTIPART_TOKEN_ORDER_MESSAGE.to_string()
            )
        );
    }
    // A token-first form with the wrong token keeps the generic refusal.
    let wrong = form(&[field("_token", "wrong"), field("title", "x")]);
    assert_eq!(
        post_form(&production, Some(&token), wrong).await,
        (
            StatusCode::FORBIDDEN,
            "Invalid or missing CSRF token".to_string()
        )
    );
}

#[tokio::test]
async fn waf_signatures_in_text_fields_are_refused_only_where_the_waf_enforces() {
    let comment = |token: &str| {
        form(&[
            field("_token", token),
            field("title", "<script>alert(document.cookie)</script>"),
        ])
    };
    let production = app(Environment::Production, false);
    let token = csrf_token(&production).await;
    assert_eq!(
        post_form(&production, Some(&token), comment(&token)).await,
        (
            StatusCode::FORBIDDEN,
            "Access Denied: Malicious pattern detected by Rullst Shield WAF.".to_string()
        )
    );
    // An exempt path is a per-route risk acceptance: fields pass unread.
    let exempt = app(Environment::Production, true);
    let token = csrf_token(&exempt).await;
    let (status, body) = post_form(&exempt, Some(&token), comment(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with("<script>"));
    // Development only reports what production would refuse.
    let development = app(Environment::Development, false);
    let (status, _) = post_form(&development, None, comment("t")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn limits_and_content_checks_answer_before_the_handler_succeeds() {
    let development = app(Environment::Development, false);
    let too_large = form(&[file_part("document", "big.pdf", &vec![b'x'; 200_001])]);
    let (status, body) = post_form(&development, None, too_large).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body, "an uploaded file exceeds its size limit");
    let image = form(&[file_part("document", "photo.pdf", b"\x89PNG\r\n\x1a\n0000")]);
    let (status, _) = post_form(&development, None, image).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let response = within(
        development.clone().oneshot(
            HttpRequest::post("/upload")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        ),
    )
    .await
    .unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        MultipartError::NotMultipart.into_response().status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
}
