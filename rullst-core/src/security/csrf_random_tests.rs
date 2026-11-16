//! CSRF randomness: tokens from a generator seeded from the operating system
//! for each token, masks from a per-thread generator that is reseeded on
//! schedule, and a failing source that never yields a value.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::failure::FailingOsSource;
use super::*;
use crate::security::csrf_middleware;
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use std::collections::HashSet;
use tower::ServiceExt;

/// A seed function that counts its calls and seeds each generator from that
/// count, so tests can tell generators apart.
fn counting_seed(calls: &std::cell::Cell<u8>) -> impl FnOnce() -> Option<StdRng> + '_ {
    move || {
        calls.set(calls.get() + 1);
        Some(StdRng::from_seed([calls.get(); 32]))
    }
}

#[test]
fn a_minted_token_is_32_random_letters_and_digits() {
    let tokens: Vec<String> = (0..64).map(|_| mint_cookie_token().unwrap()).collect();
    for token in &tokens {
        assert_eq!(token.len(), 32);
        assert!(token.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    }
    assert_eq!(tokens.iter().collect::<HashSet<_>>().len(), 64);
    // 2,048 characters over 62 symbols use nearly all of them.
    let symbols: HashSet<u8> = tokens.iter().flat_map(|token| token.bytes()).collect();
    assert!(symbols.len() >= 55, "{} symbols", symbols.len());
}

#[test]
fn masks_differ_and_are_never_all_zero() {
    let masks: Vec<[u8; 32]> = (0..64)
        .map(|_| {
            let mut mask = [0_u8; 32];
            fill_mask(&mut mask).unwrap();
            mask
        })
        .collect();
    assert!(masks.iter().all(|mask| mask.iter().any(|byte| *byte != 0)));
    assert_eq!(masks.iter().collect::<HashSet<_>>().len(), 64);
}

#[test]
fn the_first_mask_seeds_a_generator_that_later_masks_reuse() {
    let calls = std::cell::Cell::new(0);
    let mut state = None;
    let mut first = [0_u8; 32];
    let mut second = [0_u8; 32];
    fill_from(&mut state, counting_seed(&calls), &mut first).unwrap();
    fill_from(&mut state, counting_seed(&calls), &mut second).unwrap();
    assert_eq!(calls.get(), 1);
    assert_ne!(first, second);
    // Both came from the generator seeded with [1; 32], in order.
    let mut expected = StdRng::from_seed([1; 32]);
    let mut bytes = [0_u8; 64];
    expected.fill_bytes(&mut bytes);
    assert_eq!([first, second].concat(), bytes);
    assert_eq!(state.as_ref().unwrap().remaining, 65_536 - 64);
}

#[test]
fn a_generator_is_reseeded_after_64_kib_of_masks() {
    let calls = std::cell::Cell::new(0);
    let mut state = None;
    let mut mask = [0_u8; 32];
    // 2,048 masks of 32 bytes use exactly one seed's 65,536 bytes.
    for _ in 0..2_048 {
        fill_from(&mut state, counting_seed(&calls), &mut mask).unwrap();
    }
    assert_eq!((calls.get(), state.as_ref().unwrap().remaining), (1, 0));
    fill_from(&mut state, counting_seed(&calls), &mut mask).unwrap();
    assert_eq!(
        (calls.get(), state.as_ref().unwrap().remaining),
        (2, 65_504)
    );
    let mut expected = [0_u8; 32];
    StdRng::from_seed([2; 32]).fill_bytes(&mut expected);
    assert_eq!(mask, expected);
}

#[test]
fn a_mask_that_does_not_fit_the_rest_of_a_seed_reseeds() {
    let calls = std::cell::Cell::new(0);
    let mut state = Some(MaskGenerator {
        rng: StdRng::from_seed([9; 32]),
        remaining: 31,
    });
    let mut mask = [0_u8; 32];
    fill_from(&mut state, counting_seed(&calls), &mut mask).unwrap();
    assert_eq!(
        (calls.get(), state.as_ref().unwrap().remaining),
        (1, 65_504)
    );
    // A mask longer than a whole seed's budget still gets bytes.
    let mut long = vec![0_u8; 65_537];
    fill_from(&mut state, counting_seed(&calls), &mut long).unwrap();
    assert_eq!((calls.get(), state.as_ref().unwrap().remaining), (2, 0));
}

#[test]
fn a_failing_seed_yields_no_mask_and_keeps_the_state() {
    let mut state = None;
    let mut mask = [7_u8; 32];
    assert_eq!(fill_from(&mut state, || None, &mut mask), None);
    assert!(state.is_none());
    assert_eq!(mask, [7; 32]);

    state = Some(MaskGenerator {
        rng: StdRng::from_seed([3; 32]),
        remaining: 16,
    });
    assert_eq!(fill_from(&mut state, || None, &mut mask), None);
    assert_eq!(state.as_ref().unwrap().remaining, 16);
    // A generator with room never asks for a seed.
    assert_eq!(fill_from(&mut state, || None, &mut mask[..16]), Some(()));
    assert_eq!(state.as_ref().unwrap().remaining, 0);
}

#[test]
fn a_failing_operating_system_source_yields_no_token_and_no_mask() {
    {
        let _failing = FailingOsSource::start();
        assert_eq!(mint_cookie_token(), None);
        assert_eq!(fill_mask(&mut [0_u8; 32]), None);
        assert_eq!(crate::security::mask_csrf_token("Token0123456789"), None);
    }
    assert!(mint_cookie_token().is_some());
    assert!(fill_mask(&mut [0_u8; 32]).is_some());
}

fn form_app() -> Router {
    Router::new()
        .route(
            "/form",
            get(
                |Extension(token): Extension<crate::security::CsrfToken>| async move {
                    token.as_str().to_owned()
                },
            )
            .post(|| async { "written" }),
        )
        .layer(axum::middleware::from_fn(csrf_middleware))
}

#[tokio::test]
async fn a_failing_source_answers_500_without_any_token() {
    let cookie = "Token0123456789abcdefABCDEF01234";
    let requests = [
        Request::get("/form").body(Body::empty()).unwrap(),
        Request::get("/form")
            .header(header::COOKIE, format!("rullst_csrf={cookie}"))
            .body(Body::empty())
            .unwrap(),
        Request::post("/form")
            .header(header::COOKIE, format!("rullst_csrf={cookie}"))
            .header("x-csrf-token", cookie)
            .body(Body::empty())
            .unwrap(),
    ];
    let app = form_app();
    let _failing = FailingOsSource::start();
    for request in requests {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"CSRF token unavailable");
    }
}
