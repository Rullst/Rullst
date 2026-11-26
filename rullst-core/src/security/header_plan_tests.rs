//! The exact headers, in order, that the baseline's header step gives each
//! kind of response under each kind of plan.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::headers::{HeaderPlan, SecurityHeadersApplied, with_security_headers};
use crate::config::SecurityConfig;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, Response};

/// The plans: the defaults with and without HSTS, and configured values.
fn plans() -> Vec<(&'static str, HeaderPlan)> {
    let default = HeaderPlan::new(&SecurityConfig::default());
    let configured = HeaderPlan::new(&SecurityConfig {
        csp: "default-src 'self' https://cdn.example".to_string(),
        coep: "credentialless".to_string(),
        hsts_max_age_seconds: 60,
        hsts_include_subdomains: false,
        hsts_preload: true,
        ..SecurityConfig::default()
    });
    let nonce_template = HeaderPlan::new(&SecurityConfig {
        csp: "script-src 'nonce-{NONCE}'".to_string(),
        coep: "bad\nvalue".to_string(),
        ..SecurityConfig::default()
    });
    vec![
        ("default", default.clone()),
        ("over http", default.without_hsts()),
        ("configured", configured),
        ("nonce template", nonce_template),
    ]
}

/// A response before the step: a label, its headers, and whether an inner
/// layer marked it.
type Case = (&'static str, Vec<(&'static str, &'static [u8])>, bool);

/// The responses.
fn responses() -> Vec<Case> {
    vec![
        ("no headers", vec![], false),
        (
            "text",
            vec![("content-type", b"text/plain; charset=utf-8")],
            false,
        ),
        (
            "page with cookies",
            vec![
                ("content-type", b"text/html; charset=utf-8"),
                ("vary", b"HX-Request"),
                ("set-cookie", b"a=1"),
                ("set-cookie", b"b=2"),
            ],
            false,
        ),
        (
            "xhtml",
            vec![("content-type", b"Application/XHTML+XML")],
            false,
        ),
        (
            "unreadable type",
            vec![("content-type", b"text/plain\xff")],
            false,
        ),
        (
            "own frame options",
            vec![
                ("content-type", b"text/plain"),
                ("x-frame-options", b"SAMEORIGIN"),
            ],
            false,
        ),
        (
            "own cache policy",
            vec![
                ("cache-control", b"public, max-age=60"),
                ("content-type", b"text/css"),
            ],
            false,
        ),
        (
            "no-referrer among others",
            vec![
                ("referrer-policy", b"origin"),
                ("referrer-policy", b"no-referrer"),
            ],
            false,
        ),
        (
            "own referrer policy",
            vec![("referrer-policy", b"origin")],
            false,
        ),
        (
            "own csp",
            vec![("content-security-policy", b"default-src 'none'")],
            false,
        ),
        (
            "own hsts",
            vec![("strict-transport-security", b"max-age=5")],
            false,
        ),
        (
            "own coep",
            vec![("cross-origin-embedder-policy", b"unsafe-none")],
            false,
        ),
        (
            "own cross-origin policies",
            vec![
                ("permissions-policy", b"camera=(self)"),
                ("cross-origin-opener-policy", b"unsafe-none"),
                ("cross-origin-resource-policy", b"cross-origin"),
                ("x-xss-protection", b"1"),
                ("x-content-type-options", b"none"),
            ],
            false,
        ),
        (
            "nine other names",
            vec![
                ("content-type", b"application/json"),
                ("etag", b"\"1\""),
                ("last-modified", b"Wed, 01 Jan 2025 00:00:00 GMT"),
                ("x-one", b"1"),
                ("x-two", b"2"),
                ("x-three", b"3"),
                ("x-four", b"4"),
                ("x-five", b"5"),
                ("x-six", b"6"),
            ],
            false,
        ),
        ("marked", vec![("content-type", b"text/html")], true),
        (
            "marked with cache policy",
            vec![("cache-control", b"private")],
            true,
        ),
    ]
}

/// `value` with each CSP nonce replaced by `<nonce>` and any byte outside
/// printable ASCII written as `\xNN`.
fn normalized(value: &HeaderValue) -> String {
    let mut text: String = value
        .as_bytes()
        .iter()
        .map(|&byte| match byte {
            b' '..=b'~' => char::from(byte).to_string(),
            _ => format!("\\x{byte:02x}"),
        })
        .collect();
    let mut from = 0;
    while let Some(found) = text[from..].find("'nonce-") {
        let start = from + found + 7;
        let end = start + text[start..].find('\'').unwrap();
        text.replace_range(start..end, "<nonce>");
        from = start;
    }
    text
}

/// The headers after the step for every plan and response.
async fn transcript() -> String {
    let mut lines = Vec::new();
    for (plan_label, plan) in plans() {
        for (label, before, marked) in responses() {
            let mut headers = HeaderMap::new();
            for (name, value) in before {
                headers.append(
                    HeaderName::from_static(name),
                    HeaderValue::from_bytes(value).unwrap(),
                );
            }
            let response =
                with_security_headers(Request::new(Body::empty()), &plan, |_| async move {
                    let mut response = Response::new(Body::empty());
                    *response.headers_mut() = headers;
                    if marked {
                        response.extensions_mut().insert(SecurityHeadersApplied);
                    }
                    response
                })
                .await;
            lines.push(format!("{plan_label}, {label}:"));
            for (name, value) in response.headers() {
                lines.push(format!("  {name}: {}", normalized(value)));
            }
        }
    }
    lines.join("\n")
}

/// The transcript the step gave before the plan learned to append its
/// headers to a response that has none of them (13.0.0-rc.1).
const EXPECTED: &str = include_str!("header_plan_tests.txt");

#[tokio::test]
async fn every_plan_and_response_keeps_its_headers_in_order() {
    let actual = transcript().await;
    for (number, (actual, expected)) in actual.lines().zip(EXPECTED.lines()).enumerate() {
        assert_eq!(actual, expected, "line {}", number + 1);
    }
    assert_eq!(actual.lines().count(), EXPECTED.lines().count());
}

#[test]
fn only_a_response_without_any_step_header_and_few_names_takes_the_appending_path() {
    let map = |names: &[&'static str]| {
        let mut headers = HeaderMap::new();
        for name in names {
            headers.append(HeaderName::from_static(name), HeaderValue::from_static("1"));
        }
        headers
    };
    let lacks = |names: &[&'static str]| super::headers::lacks_step_headers(&map(names));
    let others = [
        "content-type",
        "set-cookie",
        "vary",
        "etag",
        "x-one",
        "x-two",
        "x-three",
        "x-four",
        "x-five",
    ];
    assert!(lacks(&[]));
    assert!(lacks(&others[..1]));
    // Names, not values: a second cookie is still eight names.
    assert!(lacks(&[&others[..8], &["set-cookie"]].concat()));
    assert!(!lacks(&others));
    for name in [
        "cache-control",
        "x-frame-options",
        "x-content-type-options",
        "x-xss-protection",
        "permissions-policy",
        "cross-origin-opener-policy",
        "cross-origin-resource-policy",
        "strict-transport-security",
        "referrer-policy",
        "cross-origin-embedder-policy",
        "content-security-policy",
    ] {
        assert!(!lacks(&[name]), "{name}");
        assert!(!lacks(&[&others[..7], &[name]].concat()), "{name}");
    }
}
