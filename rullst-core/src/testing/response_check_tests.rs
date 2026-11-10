//! `TestResponse::assert_security_headers` and `assert_html_hygiene`.

use super::*;
use axum::response::IntoResponse;
use axum::routing::get;

/// A response with exactly `headers` and `body`.
fn respond(headers: &[(&'static str, &'static str)], body: &'static str) -> TestApp {
    let headers = headers.to_vec();
    TestApp::new(Router::new().route(
        "/",
        get(move || {
            let headers = headers.clone();
            async move {
                let mut response = body.into_response();
                let mut seen = Vec::new();
                for (name, value) in headers {
                    let (name, value) = (
                        HeaderName::from_static(name),
                        HeaderValue::from_static(value),
                    );
                    // The first value replaces the handler's own header.
                    if seen.contains(&name) {
                        response.headers_mut().append(name, value);
                    } else {
                        seen.push(name.clone());
                        response.headers_mut().insert(name, value);
                    }
                }
                response
            }
        }),
    ))
}

const NOSNIFF: (&str, &str) = ("x-content-type-options", "nosniff");
const DENY: (&str, &str) = ("x-frame-options", "DENY");
const CSP: (&str, &str) = ("content-security-policy", "default-src 'self'");

#[tokio::test]
async fn rullst_pages_pass_both_checks() {
    async fn page(htmx: crate::htmx::HtmxRequest) -> impl IntoResponse {
        let name = "<Rullst & co>";
        let content = crate::html! {
            <main><h1 title={name}>"Hello " {name}</h1>
                <button type="button" disabled?={true} aria-pressed={false}>"Go"</button>
            </main>
        };
        crate::htmx::render_page_with_lang(&htmx, "en", "Home", content)
    }
    let app = TestApp::new(
        Router::new()
            .route("/", get(page))
            .layer(axum::middleware::from_fn(
                crate::security::headers_middleware,
            )),
    );
    let response = app.get("/").await;
    response.assert_status(200).assert_see("nonce=\"");
    response.assert_security_headers().assert_html_hygiene();
}

#[tokio::test]
async fn security_headers_accept_either_frame_protection() {
    for headers in [
        vec![CSP, NOSNIFF, DENY],
        vec![CSP, NOSNIFF, ("x-frame-options", " sameorigin ")],
        vec![
            (
                "content-security-policy",
                "default-src 'self'; FRAME-ANCESTORS 'none'",
            ),
            NOSNIFF,
        ],
        vec![
            ("content-security-policy", "default-src 'self'"),
            ("content-security-policy", "frame-ancestors 'self'"),
            ("x-content-type-options", " NoSniff "),
        ],
    ] {
        respond(&headers, "ok")
            .get("/")
            .await
            .assert_security_headers();
    }
}

#[tokio::test]
#[should_panic(expected = "Expected a Content-Security-Policy header")]
async fn security_headers_need_a_policy() {
    respond(&[NOSNIFF, DENY], "ok")
        .get("/")
        .await
        .assert_security_headers();
}

#[tokio::test]
#[should_panic(expected = "Expected a Content-Security-Policy header")]
async fn security_headers_need_a_non_empty_policy() {
    respond(&[("content-security-policy", " "), NOSNIFF, DENY], "ok")
        .get("/")
        .await
        .assert_security_headers();
}

#[tokio::test]
#[should_panic(expected = "Expected X-Content-Type-Options: nosniff, but got None")]
async fn security_headers_need_nosniff() {
    respond(&[CSP, DENY], "ok")
        .get("/")
        .await
        .assert_security_headers();
}

#[tokio::test]
#[should_panic(expected = "Expected X-Content-Type-Options: nosniff, but got Some(\"sniff\")")]
async fn security_headers_need_the_nosniff_value() {
    respond(&[CSP, ("x-content-type-options", "sniff"), DENY], "ok")
        .get("/")
        .await
        .assert_security_headers();
}

#[tokio::test]
#[should_panic(expected = "Expected frame protection")]
async fn security_headers_need_frame_protection() {
    respond(
        &[
            CSP,
            NOSNIFF,
            ("x-frame-options", "ALLOW-FROM https://a.example"),
        ],
        "ok",
    )
    .get("/")
    .await
    .assert_security_headers();
}

#[tokio::test]
#[should_panic(expected = "Expected frame protection")]
async fn a_frame_ancestors_source_is_not_the_directive() {
    respond(
        &[
            ("content-security-policy", "default-src frame-ancestors"),
            NOSNIFF,
        ],
        "ok",
    )
    .get("/")
    .await
    .assert_security_headers();
}

#[tokio::test]
async fn hygiene_checks_only_html_responses() {
    let broken = "<p title=\"a\"b>&lt;div&gt;</p>";
    respond(&[("content-type", "application/json")], broken)
        .get("/")
        .await
        .assert_html_hygiene();
    // A plain-text body is not HTML either.
    respond(&[], broken).get("/").await.assert_html_hygiene();
    respond(&[("content-type", "text/html")], "<p>ok</p>")
        .get("/")
        .await
        .assert_html_hygiene();
}

#[tokio::test]
#[should_panic(expected = "but found 2:\nline 1: malformed markup")]
async fn hygiene_lists_every_issue_of_an_html_page() {
    respond(
        &[("content-type", " Text/HTML; charset=utf-8")],
        "<p title=\"a\"b>&lt;div&gt;</p>",
    )
    .get("/")
    .await
    .assert_html_hygiene();
}

#[tokio::test]
#[should_panic(expected = "line 1: escaped twice")]
async fn hygiene_checks_xhtml_responses() {
    respond(&[("content-type", "application/xhtml+xml")], "<p>&lt;b</p>")
        .get("/")
        .await
        .assert_html_hygiene();
}

#[tokio::test]
#[should_panic(expected = "CSP nonce is blocked")]
async fn hygiene_uses_every_policy_of_the_response() {
    respond(
        &[
            ("content-type", "text/html"),
            ("content-security-policy", "img-src 'self'"),
            ("content-security-policy", "script-src 'nonce-a'"),
        ],
        "<script>run()</script>",
    )
    .get("/")
    .await
    .assert_html_hygiene();
}

#[tokio::test]
async fn hygiene_without_a_policy_skips_the_csp_checks() {
    respond(
        &[("content-type", "text/html")],
        "<script>run()</script><p style=\"x\">x</p>",
    )
    .get("/")
    .await
    .assert_html_hygiene();
}
