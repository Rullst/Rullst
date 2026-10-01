#![allow(clippy::expect_used)]

use super::{card_mask_count, is_textual_response, mask_card_numbers, mask_json_strings, mask_pii};
use std::time::{Duration, Instant};

/// The former per-position rescan, kept as a differential oracle for the
/// masking result. It is quadratic on long runs; only call it on short input.
fn legacy_mask_card_numbers(chars: &mut [char]) {
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut digit_indices = vec![i];
            let mut j = i + 1;
            let mut non_digits = 0;
            while j < chars.len() && non_digits < 3 {
                let c = chars[j];
                if c.is_ascii_digit() {
                    digit_indices.push(j);
                    non_digits = 0;
                } else if c == ' ' || c == '-' {
                    non_digits += 1;
                } else {
                    break;
                }
                j += 1;
            }
            if let Some(mask_count) = card_mask_count(digit_indices.len()) {
                for index in digit_indices.iter().take(mask_count) {
                    chars[*index] = '*';
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
}

fn assert_matches_legacy(input: &str) {
    let mut expected: Vec<char> = input.chars().collect();
    legacy_mask_card_numbers(&mut expected);
    let mut actual: Vec<char> = input.chars().collect();
    mask_card_numbers(&mut actual);
    assert_eq!(
        actual.iter().collect::<String>(),
        expected.iter().collect::<String>(),
        "input: {input:?}"
    );
}

#[test]
fn long_digit_runs_keep_legacy_trailing_window_masking() {
    for (input, expected) in [
        ("1234567890123456789", "***************6789"),
        ("12345678901234567890", "1***************7890"),
        ("1234567890123456789012345", "123456***************2345"),
        ("x12345678901234567890123 y", "x1234***************0123 y"),
        (
            "1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1",
            "1 1 * * * * * * * * * * * * * * * 1 1 1 1",
        ),
        (
            "1-2-3-4-5-6-7-8-9-0-1-2-3-4-5-6-7-8-9-0-1-2",
            "1-2-3-*-*-*-*-*-*-*-*-*-*-*-*-*-*-*-9-0-1-2",
        ),
        (
            "1234  5678 1234 5678 9999 1111",
            "1234  5*** **** **** **** 1111",
        ),
        (
            "1234   5678123456789999 1111",
            "1234   5*************** 1111",
        ),
    ] {
        assert_eq!(mask_pii(input), expected, "input: {input:?}");
    }
}

#[test]
fn linear_card_scan_matches_legacy_rescan() {
    for digits in 1..=45 {
        for separator in ["", " ", "-", "  ", "- ", "   ", "x"] {
            let run = vec!["7"; digits].join(separator);
            assert_matches_legacy(&run);
            assert_matches_legacy(&format!("a{run}  -12 34@b.co"));
        }
    }

    let alphabet: Vec<char> = "01234567890123456789012345678901234567  --- x@.*"
        .chars()
        .collect();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..3_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let len = (state % 64) as usize;
        let input: String = (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                alphabet[(state % alphabet.len() as u64) as usize]
            })
            .collect();
        assert_matches_legacy(&input);
    }
}

#[test]
fn over_long_digit_runs_are_masked_in_linear_time() {
    // Linear scanning finishes in milliseconds. The former per-position
    // rescan performs roughly n^2 / 2 steps (about 2 * 10^10 here) plus one
    // allocation per start position, which takes minutes.
    let started = Instant::now();

    let digits = "7".repeat(200_000);
    let masked = mask_pii(&digits);
    assert_eq!(masked.len(), digits.len());
    assert_eq!(&masked[..199_981], &digits[..199_981]);
    assert_eq!(&masked[199_981..], "***************7777");

    let spaced = "1 ".repeat(100_000);
    let masked = mask_pii(&spaced);
    assert_eq!(masked.len(), spaced.len());
    let tail_start = spaced.len() - 38;
    assert_eq!(&masked[..tail_start], &spaced[..tail_start]);
    assert_eq!(
        &masked[tail_start..],
        format!("{}1 1 1 1 ", "* ".repeat(15))
    );

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "masking 400,000 characters took {:?}",
        started.elapsed()
    );
}

#[test]
fn json_masking_rewrites_strings_but_never_numbers() {
    let body = r#"{"created_at":1727712000000,"id":1234567890123456789,"card":"4111 1111 1111 1111","note":"say \"hi\" to ana@example.com","ids":[9007199254740993]}"#;
    let masked = mask_json_strings(body);
    assert_eq!(
        masked,
        r#"{"created_at":1727712000000,"id":1234567890123456789,"card":"**** **** **** 1111","note":"say \"hi\" to a**@example.com","ids":[9007199254740993]}"#
    );
    let value: serde_json::Value = serde_json::from_str(&masked).expect("still valid JSON");
    assert_eq!(value["created_at"], 1_727_712_000_000_u64);
    // The plain-text masker still masks the same digit run.
    assert!(mask_pii(body).contains("*********0000"));
}

#[test]
fn json_escape_sequences_never_join_a_digit_run() {
    // `\u2014` (em dash) and serde_json's `\u0019` are not digits: neither
    // body holds a 13-digit run, so both stay byte-for-byte unchanged.
    let em_dash = r#"{"note":"Pedido \u2014 123456789"}"#;
    assert_eq!(mask_json_strings(em_dash), em_dash);
    let control = serde_json::json!({ "note": "\u{19}1234567890" }).to_string();
    assert!(control.contains(r"\u0019"));
    assert_eq!(mask_json_strings(&control), control);

    // Escaped digits and `@` still count, and the masked output stays valid.
    let escaped =
        r#"{"card":"\u0034111 1111 1111 1111","email":"ana\u0040example.com","q":"\"5\" \\ \/"}"#;
    let masked = mask_json_strings(escaped);
    assert_eq!(
        masked,
        r#"{"card":"**** **** **** 1111","email":"a**\u0040example.com","q":"\"5\" \\ \/"}"#
    );
    let value: serde_json::Value = serde_json::from_str(&masked).expect("still valid JSON");
    assert_eq!(value["email"], "a**@example.com");

    // Lone surrogates and malformed escapes are copied unchanged.
    assert_eq!(
        mask_json_strings(r#"["\ud83d 4111 1111 1111 1111"]"#),
        r#"["\ud83d **** **** **** 1111"]"#
    );
    let malformed = r#"["\uZZZZ","\x","\u12"]"#;
    assert_eq!(mask_json_strings(malformed), malformed);
}

#[tokio::test]
async fn middleware_keeps_json_numbers_valid() {
    use axum::http::{Request, header};
    use tower::ServiceExt;

    let app = axum::Router::new()
        .route(
            "/json",
            axum::routing::get(|| async {
                (
                    [(header::CONTENT_TYPE, "application/problem+json")],
                    r#"{"ts":1727712000000,"email":"ana@example.com"}"#,
                )
            }),
        )
        .layer(axum::middleware::from_fn(super::pii_masking_middleware));
    let response = app
        .oneshot(
            Request::get("/json")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let body = axum::body::to_bytes(response.into_body(), 1_024)
        .await
        .expect("body");
    assert_eq!(
        body.as_ref(),
        br#"{"ts":1727712000000,"email":"a**@example.com"}"#
    );
}

#[test]
fn versioned_urls_and_asset_names_are_not_email_addresses() {
    for unchanged in [
        r#"<script src="https://unpkg.com/htmx.org@2.0.4"></script>"#,
        "https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.67.0",
        r#"<img src="/static/logo@2x.png" srcset="icon@3x.webp 3x">"#,
        "import x from 'pkg@1.2.3/dist/index.js'",
    ] {
        assert_eq!(mask_pii(unchanged), unchanged);
    }
    for (address, masked) in [
        (
            "contact ana.silva@example.com.br today",
            "contact a********@example.com.br today",
        ),
        ("mail user@163.com.", "mail u***@163.com."),
        ("to: jo@xn--80ak6aa92e.com", "to: j*@xn--80ak6aa92e.com"),
    ] {
        assert_eq!(mask_pii(address), masked);
    }
}

#[test]
fn textual_responses_are_recognized_in_any_ascii_case() {
    use axum::http::{HeaderMap, HeaderValue, header};
    for media_type in [
        "application/problem+JSON",
        "Application/Vnd.Api+Json; charset=utf-8",
        "APPLICATION/JSON",
        "Application/Atom+XML",
        "Text/HTML",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(media_type));
        assert!(is_textual_response(&headers), "{media_type}");
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("Text/Event-Stream"),
    );
    assert!(!is_textual_response(&headers));
}

/// Runs one response through the PII middleware.
async fn through_pii_layer(
    status: axum::http::StatusCode,
    headers: Vec<(axum::http::HeaderName, &'static str)>,
    body: &'static str,
) -> axum::response::Response {
    use axum::http::{HeaderValue, Request};
    use tower::ServiceExt;

    let app = axum::Router::new()
        .route(
            "/range",
            axum::routing::get(move || async move {
                let mut response = axum::response::Response::new(axum::body::Body::from(body));
                *response.status_mut() = status;
                for (name, value) in headers {
                    response
                        .headers_mut()
                        .insert(name, HeaderValue::from_static(value));
                }
                response
            }),
        )
        .layer(axum::middleware::from_fn(super::pii_masking_middleware));
    app.oneshot(
        Request::get("/range")
            .body(axum::body::Body::empty())
            .expect("request"),
    )
    .await
    .expect("response")
}

async fn response_text(response: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    String::from_utf8(bytes.to_vec()).expect("UTF-8 body")
}

#[tokio::test]
async fn masked_partial_content_is_withheld_instead_of_rewritten() {
    use axum::http::{HeaderValue, StatusCode, header};

    let response = through_pii_layer(
        StatusCode::PARTIAL_CONTENT,
        vec![
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CONTENT_RANGE, "bytes 100-129/5000"),
            (header::ACCEPT_RANGES, "bytes"),
        ],
        "contact ana.silva@example.com.",
    )
    .await;

    // Never a 206 whose body no longer matches its Content-Range, and never
    // the unmasked range.
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(response.headers().get(header::CONTENT_RANGE).is_none());
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&HeaderValue::from_static("no-store"))
    );
    assert!(!response_text(response).await.contains("ana.silva"));
}

#[tokio::test]
async fn clean_partial_content_passes_through_untouched() {
    use axum::http::{HeaderValue, StatusCode, header};

    let response = through_pii_layer(
        StatusCode::PARTIAL_CONTENT,
        vec![
            (header::CONTENT_TYPE, "application/json"),
            (header::CONTENT_RANGE, "bytes 0-25/5000"),
            (header::ETAG, "\"v1\""),
        ],
        r#"{"ts":1727712000000,"n":1}"#,
    )
    .await;

    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        response.headers().get(header::CONTENT_RANGE),
        Some(&HeaderValue::from_static("bytes 0-25/5000"))
    );
    assert_eq!(
        response.headers().get(header::ETAG),
        Some(&HeaderValue::from_static("\"v1\""))
    );
    assert_eq!(
        response_text(response).await,
        r#"{"ts":1727712000000,"n":1}"#
    );
}
