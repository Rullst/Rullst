use super::*;

const SECRET: &[u8] = b"rullst-mail-test-key-32-bytes-minimum-2026";
const NOW: u64 = 1_800_000_000;

#[test]
fn tokens_are_purpose_bound_constant_time_verified_and_fresh() {
    let token =
        TrackingEngine::try_generate_open_token(SECRET, "user@example.com", "onboarding_1", NOW)
            .expect("strong secret");
    let event =
        TrackingEngine::verify_open_token_at(SECRET, &token, NOW + 60, Duration::from_secs(120))
            .expect("fresh token");
    assert_eq!(event.email, "user@example.com");

    assert_eq!(
        TrackingEngine::verify_open_token_at(
            b"a weak secret",
            &token,
            NOW,
            Duration::from_secs(120)
        ),
        Err(TrackingError::WeakSecret)
    );
    assert_eq!(
        TrackingEngine::verify_click_token_at(SECRET, &token, NOW, Duration::from_secs(120)),
        Err(TrackingError::InvalidSignature)
    );
}

#[test]
fn rejects_expired_and_future_tokens() {
    let token =
        TrackingEngine::try_generate_open_token(SECRET, "user@example.com", "campaign", NOW)
            .expect("token");
    assert_eq!(
        TrackingEngine::verify_open_token_at(SECRET, &token, NOW + 61, Duration::from_secs(60)),
        Err(TrackingError::Expired)
    );

    let future = TrackingEngine::try_generate_open_token(
        SECRET,
        "user@example.com",
        "campaign",
        NOW + ALLOWED_CLOCK_SKEW_SECS + 1,
    )
    .expect("token");
    assert_eq!(
        TrackingEngine::verify_open_token_at(SECRET, &future, NOW, Duration::from_secs(600)),
        Err(TrackingError::NotYetValid)
    );
}

#[test]
fn replay_verifier_consumes_once() {
    let verifier = TrackingVerifier::new(Duration::from_secs(60), 8).expect("policy");
    let token = TrackingEngine::try_generate_click_token(
        SECRET,
        "user@example.com",
        "https://rullst.dev",
        NOW,
    )
    .expect("token");
    assert!(verifier.verify_click_once(SECRET, &token, NOW).is_ok());
    assert_eq!(
        verifier.verify_click_once(SECRET, &token, NOW),
        Err(TrackingError::ReplayDetected)
    );
}

#[test]
fn pixel_and_link_rewrite_validate_tracker_url() {
    let html = r##"<body><a href="https://example.com">go</a></body>"##;
    let rewritten = TrackingEngine::try_rewrite_links(
        html,
        "https://track.example.com",
        SECRET,
        "user@example.com",
        NOW,
    )
    .expect("valid tracker");
    assert!(rewritten.contains("https://track.example.com/track/click/"));
    assert_eq!(
        TrackingEngine::try_inject_open_pixel(html, "javascript:alert(1)"),
        Err(TrackingError::InvalidTrackerUrl)
    );
    assert_eq!(PIXEL_1X1_GIF.len(), 43);
}
