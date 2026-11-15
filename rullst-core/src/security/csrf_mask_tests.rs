//! Per-response CSRF masking: every embedded value differs, each one and the
//! cookie value itself verify, and nothing else does.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::security::{CsrfToken, csrf_middleware};
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use tower::ServiceExt;

const COOKIE: &str = "MaskedTokenFixture0123456789abcd";

fn matches(candidate: &[u8], cookie: &str) -> bool {
    csrf_proof_matches(candidate, cookie).into()
}

/// The mask and the masked token of `value`.
fn halves(value: &str) -> (Vec<u8>, Vec<u8>) {
    let decoded = URL_SAFE_NO_PAD.decode(value).unwrap();
    let (mask, cipher) = decoded.split_at(decoded.len() / 2);
    (mask.to_vec(), cipher.to_vec())
}

#[test]
fn a_masked_value_is_mask_then_token_xor_mask_in_url_safe_base64() {
    let masked = mask_csrf_token(COOKIE).unwrap();
    assert_eq!(masked.len(), 86);
    assert!(
        masked
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')),
        "{masked}"
    );
    let (mask, cipher) = halves(&masked);
    assert_eq!(mask.len(), 32);
    let token = mask
        .iter()
        .zip(&cipher)
        .map(|(m, c)| m ^ c)
        .collect::<Vec<_>>();
    assert_eq!(token, COOKIE.as_bytes());
    // The longest cookie value fits the longest masked value.
    let longest = "a".repeat(128);
    assert_eq!(mask_csrf_token(&longest).unwrap().len(), 342);
    assert_eq!(MAX_MASKED_TOKEN_BYTES, 342);
    assert_eq!(mask_csrf_token("x").unwrap().len(), 3);
}

#[test]
fn every_mask_is_fresh_and_every_masked_value_verifies() {
    let values = (0..64)
        .map(|_| mask_csrf_token(COOKIE).unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(values.len(), 64, "a mask repeated");
    for value in &values {
        assert!(matches(value.as_bytes(), COOKIE), "{value}");
        let (mask, _) = halves(value);
        assert_ne!(mask, vec![0_u8; 32], "an all-zero mask embeds the token");
    }
}

#[test]
fn the_cookie_value_itself_still_verifies_and_nothing_else_does() {
    assert!(matches(COOKIE.as_bytes(), COOKIE));
    let masked = mask_csrf_token(COOKIE).unwrap();
    let other = mask_csrf_token("AnotherTokenFixture0123456789abc").unwrap();
    let rejected = [
        String::new(),
        COOKIE[..31].to_owned(),
        format!("{COOKIE}x"),
        COOKIE.to_ascii_lowercase(),
        // Another cookie's masked value.
        other,
        // Too short, too long, not base64, padded.
        masked[..85].to_owned(),
        format!("{masked}A"),
        format!("{}*", &masked[..85]),
        format!("{masked}=="),
        // The masked value of a token of another length.
        mask_csrf_token(&COOKIE[..31]).unwrap(),
        "A".repeat(4096),
    ];
    for candidate in rejected {
        assert!(!matches(candidate.as_bytes(), COOKIE), "{candidate}");
    }
}

#[test]
fn a_change_anywhere_in_a_masked_value_fails_verification() {
    let masked = mask_csrf_token(COOKIE).unwrap();
    let (mask, cipher) = halves(&masked);
    // One flipped bit in the first, a middle or the last byte of each half.
    for index in [0, 15, 31] {
        for (flip_mask, flip_cipher) in [(true, false), (false, true)] {
            let mut mask = mask.clone();
            let mut cipher = cipher.clone();
            if flip_mask {
                mask[index] ^= 1;
            }
            if flip_cipher {
                cipher[index] ^= 0x80;
            }
            let tampered = URL_SAFE_NO_PAD.encode([mask, cipher].concat());
            assert!(!matches(tampered.as_bytes(), COOKIE), "byte {index}");
        }
    }
    // Flipping the same bit of both halves keeps the token: the mask is not
    // authenticated, only the unmasked token is compared.
    let (mut mask, mut cipher) = (mask, cipher);
    mask[7] ^= 0x20;
    cipher[7] ^= 0x20;
    let remasked = URL_SAFE_NO_PAD.encode([mask, cipher].concat());
    assert!(matches(remasked.as_bytes(), COOKIE));
}

#[test]
fn verification_is_a_constant_time_choice() {
    // `subtle::Choice` cannot be branched on without an explicit conversion,
    // and the token bytes are compared with `ConstantTimeEq` over their whole
    // length: a value differing in the first or the last byte is rejected by
    // the same comparison.
    let masked = mask_csrf_token(COOKIE).unwrap();
    let choice: Choice = csrf_proof_matches(masked.as_bytes(), COOKIE);
    assert_eq!(choice.unwrap_u8(), 1);
    let mut first = COOKIE.as_bytes().to_vec();
    first[0] ^= 1;
    let mut last = COOKIE.as_bytes().to_vec();
    last[31] ^= 1;
    for raw in [first, last] {
        assert_eq!(csrf_proof_matches(&raw, COOKIE).unwrap_u8(), 0);
        let remasked = mask_csrf_token(std::str::from_utf8(&raw).unwrap()).unwrap();
        assert_eq!(
            csrf_proof_matches(remasked.as_bytes(), COOKIE).unwrap_u8(),
            0
        );
    }
}

#[test]
fn only_valid_cookie_values_are_masked() {
    for invalid in ["", "has space", "semi;colon", "ünïcode", &"a".repeat(129)] {
        assert_eq!(mask_csrf_token(invalid), None, "{invalid:?}");
        assert!(!valid_cookie_token(invalid.as_bytes()), "{invalid:?}");
    }
    for valid in ["a", "A-b_9", &"z".repeat(128)] {
        assert!(valid_cookie_token(valid.as_bytes()), "{valid}");
        assert!(mask_csrf_token(valid).is_some(), "{valid}");
    }
}

fn page_app() -> Router {
    Router::new()
        .route(
            "/form",
            get(|Extension(token): Extension<CsrfToken>| async move { token.as_str().to_owned() })
                .post(|Extension(token): Extension<CsrfToken>| async move {
                    token.as_str().to_owned()
                }),
        )
        .layer(axum::middleware::from_fn(csrf_middleware))
}

async fn text(response: axum::response::Response) -> String {
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

async fn page(app: &Router, cookie: Option<&str>) -> (String, Option<String>) {
    let mut request = Request::get("/form");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, format!("rullst_csrf={cookie}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let set = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok()?.strip_prefix("rullst_csrf="))
        .and_then(|value| value.split(';').next())
        .map(ToOwned::to_owned);
    (text(response).await, set)
}

fn post(cookie: &str, header_token: Option<&str>, form: Option<&str>) -> Request<Body> {
    let mut request =
        Request::post("/form").header(header::COOKIE, format!("rullst_csrf={cookie}"));
    if let Some(token) = header_token {
        request = request.header("X-CSRF-Token", token);
    }
    match form {
        Some(form) => request
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(form.to_owned()))
            .unwrap(),
        None => request.body(Body::empty()).unwrap(),
    }
}

#[tokio::test]
async fn two_responses_embed_different_values_that_both_verify() {
    let app = page_app();
    let (first, cookie) = page(&app, None).await;
    let cookie = cookie.expect("first visit sets the cookie");
    assert_eq!(cookie.len(), 32);
    assert_ne!(first, cookie, "the page embeds the cookie value");
    let (second, set) = page(&app, Some(&cookie)).await;
    assert_eq!(set, None);
    assert_ne!(first, second);
    for embedded in [&first, &second] {
        assert!(!embedded.contains(&cookie));
        let response = app
            .clone()
            .oneshot(post(&cookie, Some(embedded), None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{embedded}");
        // The handler re-rendering the form gets a fresh masked value.
        let echoed = text(response).await;
        assert_ne!(&echoed, embedded);
        assert!(matches(echoed.as_bytes(), &cookie));
        let response = app
            .clone()
            .oneshot(post(
                &cookie,
                None,
                Some(&format!("title=x&_token={embedded}")),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{embedded}");
        assert!(matches(text(response).await.as_bytes(), &cookie));
    }
}

#[tokio::test]
async fn the_raw_cookie_value_verifies_and_tampered_values_do_not() {
    let app = page_app();
    let (embedded, _) = page(&app, Some(COOKIE)).await;
    for (header_token, form) in [
        (Some(COOKIE), None),
        (None, Some(format!("_token={COOKIE}"))),
    ] {
        let response = app
            .clone()
            .oneshot(post(COOKIE, header_token, form.as_deref()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let mut tampered = embedded.clone().into_bytes();
    tampered[40] = if tampered[40] == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(tampered).unwrap();
    let foreign = mask_csrf_token("AnotherTokenFixture0123456789abc").unwrap();
    for bad in [
        tampered.as_str(),
        &embedded[..60],
        &format!("{embedded}{embedded}"),
        foreign.as_str(),
    ] {
        let header = app
            .clone()
            .oneshot(post(COOKIE, Some(bad), None))
            .await
            .unwrap();
        assert_eq!(header.status(), StatusCode::FORBIDDEN, "{bad}");
        let form = app
            .clone()
            .oneshot(post(COOKIE, None, Some(&format!("_token={bad}"))))
            .await
            .unwrap();
        assert_eq!(form.status(), StatusCode::FORBIDDEN, "{bad}");
    }
}

#[tokio::test]
async fn a_multipart_token_field_accepts_masked_and_raw_values() {
    let app = Router::new()
        .route(
            "/upload",
            axum::routing::post(|Extension(token): Extension<CsrfToken>| async move {
                token.as_str().to_owned()
            }),
        )
        .layer(axum::middleware::from_fn(csrf_middleware));
    let longest = "L".repeat(128);
    let cases = [
        (
            COOKIE.to_owned(),
            mask_csrf_token(COOKIE).unwrap(),
            StatusCode::OK,
        ),
        (COOKIE.to_owned(), COOKIE.to_owned(), StatusCode::OK),
        // The longest masked value, 342 bytes, fits the multipart bound.
        (
            longest.clone(),
            mask_csrf_token(&longest).unwrap(),
            StatusCode::OK,
        ),
        (
            COOKIE.to_owned(),
            mask_csrf_token("AnotherTokenFixture0123456789abc").unwrap(),
            StatusCode::FORBIDDEN,
        ),
    ];
    for (cookie, field, expected) in cases {
        let body = format!(
            "--RullstForm\r\nContent-Disposition: form-data; name=\"_token\"\r\n\r\n{field}\r\n--RullstForm\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"a.png\"\r\nContent-Type: image/png\r\n\r\npng\r\n--RullstForm--\r\n"
        );
        let response = app
            .clone()
            .oneshot(
                Request::post("/upload")
                    .header(header::COOKIE, format!("rullst_csrf={cookie}"))
                    .header(
                        header::CONTENT_TYPE,
                        "multipart/form-data; boundary=RullstForm",
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{field}");
        if expected == StatusCode::OK {
            assert!(matches(text(response).await.as_bytes(), &cookie));
        }
    }
}
