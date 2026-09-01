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

#[test]
fn click_tokens_round_trip_and_malformed_tokens_fail_closed() {
    let token = TrackingEngine::try_generate_click_token(
        SECRET,
        "reader@example.com",
        "https://rullst.dev/guide",
        NOW,
    )
    .expect("click token");
    let event =
        TrackingEngine::verify_click_token_at(SECRET, &token, NOW + 10, Duration::from_secs(60))
            .expect("fresh click token");
    assert_eq!(event.email, "reader@example.com");
    assert_eq!(event.target_url, "https://rullst.dev/guide");

    for malformed in ["", "v1.payload.signature", "v2.only-two", "v2.a.b.extra"] {
        assert_eq!(
            TrackingEngine::verify_click_token_at(SECRET, malformed, NOW, Duration::from_secs(60)),
            Err(TrackingError::InvalidFormat)
        );
    }

    let mut parts = token.split('.');
    let version = parts.next().expect("version");
    let payload = parts.next().expect("payload");
    let mut signature = parts.next().expect("signature").as_bytes().to_vec();
    signature[0] = if signature[0] == b'A' { b'B' } else { b'A' };
    let tampered = format!(
        "{version}.{payload}.{}",
        String::from_utf8(signature).expect("ASCII signature")
    );
    assert_eq!(
        TrackingEngine::verify_click_token_at(SECRET, &tampered, NOW, Duration::from_secs(60)),
        Err(TrackingError::InvalidSignature)
    );
}

#[test]
fn signed_but_wrong_payloads_are_rejected_after_authentication() {
    let token = sign_event(
        SECRET,
        OPEN_PURPOSE,
        &serde_json::json!("not an open event"),
    )
    .expect("signed fixture");
    assert!(matches!(
        TrackingEngine::verify_open_token_at(SECRET, &token, NOW, Duration::from_secs(60)),
        Err(TrackingError::PayloadError(_))
    ));

    assert_eq!(
        TrackingEngine::verify_open_token_at(SECRET, "v2.***.***", NOW, Duration::from_secs(60)),
        Err(TrackingError::InvalidFormat)
    );
    assert_eq!(
        TrackingEngine::try_generate_open_token(&[b'a'; 64], "user@example.com", "campaign", NOW),
        Err(TrackingError::WeakSecret)
    );
    let valid =
        TrackingEngine::try_generate_open_token(SECRET, "user@example.com", "campaign", NOW)
            .expect("valid open token");
    assert_eq!(
        TrackingEngine::verify_open_token_at(SECRET, &valid, NOW, Duration::ZERO),
        Err(TrackingError::InvalidPolicy)
    );
}

#[test]
fn replay_store_is_bounded_and_discards_expired_entries() {
    assert!(matches!(
        TrackingVerifier::new(Duration::ZERO, 1),
        Err(TrackingError::InvalidPolicy)
    ));
    assert!(matches!(
        TrackingVerifier::new(Duration::from_secs(60), 0),
        Err(TrackingError::InvalidPolicy)
    ));

    let verifier = TrackingVerifier::new(Duration::from_secs(60), 1).expect("bounded verifier");
    let first = TrackingEngine::try_generate_open_token(SECRET, "first@example.com", "one", NOW)
        .expect("first token");
    let second = TrackingEngine::try_generate_open_token(SECRET, "second@example.com", "two", NOW)
        .expect("second token");
    assert!(verifier.verify_open_once(SECRET, &first, NOW).is_ok());
    assert_eq!(
        verifier.verify_open_once(SECRET, &second, NOW),
        Err(TrackingError::ReplayStoreUnavailable)
    );

    let replacement = TrackingEngine::try_generate_open_token(
        SECRET,
        "replacement@example.com",
        "three",
        NOW + 61,
    )
    .expect("replacement token");
    assert!(
        verifier
            .verify_open_once(SECRET, &replacement, NOW + 61)
            .is_ok()
    );
}

#[test]
fn pixel_and_link_helpers_preserve_non_http_content() {
    let html = r#"<BODY><a href="/local">local</a><a href="mailto:a@example.com">mail</a></BODY>"#;
    let rewritten = TrackingEngine::try_rewrite_links(
        html,
        "https://track.example.com/",
        SECRET,
        "user@example.com",
        NOW,
    )
    .expect("valid rewrite");
    assert!(rewritten.contains("href=\"/local\""));
    assert!(rewritten.contains("href=\"mailto:a@example.com\""));

    let incomplete = r#"<a href="https://example.com"#;
    assert_eq!(
        TrackingEngine::try_rewrite_links(
            incomplete,
            "https://track.example.com",
            SECRET,
            "user@example.com",
            NOW
        )
        .expect("incomplete markup remains bounded"),
        incomplete
    );

    let with_body = TrackingEngine::try_inject_open_pixel(
        "<HTML><BODY>hello</BODY></HTML>",
        "https://track.example.com/open?campaign=a&source=b",
    )
    .expect("pixel before body close");
    assert!(with_body.contains("campaign=a&amp;source=b"));
    assert!(with_body.find("<img").unwrap() < with_body.find("</BODY>").unwrap());

    let fragment =
        TrackingEngine::try_inject_open_pixel("<p>fragment</p>", "https://track.example.com/open")
            .expect("pixel appended to fragment");
    assert!(fragment.ends_with(" />"));
}

#[test]
#[allow(deprecated)]
fn compatibility_helpers_fail_closed_for_invalid_configuration() {
    assert!(TrackingEngine::generate_open_token(b"weak", "u@example.com", "c", NOW).is_empty());
    assert!(
        TrackingEngine::generate_click_token(b"weak", "u@example.com", "https://rullst.dev", NOW)
            .is_empty()
    );
    assert_eq!(
        TrackingEngine::inject_open_pixel("<p>safe</p>", "javascript:alert(1)"),
        "<p>safe</p>"
    );
    assert_eq!(
        TrackingEngine::rewrite_links(
            r#"<a href="https://rullst.dev">safe</a>"#,
            "not a URL",
            SECRET,
            "u@example.com",
            NOW
        ),
        r#"<a href="https://rullst.dev">safe</a>"#
    );
}

#[test]
fn tracking_errors_have_stable_non_secret_messages() {
    let errors = [
        TrackingError::WeakSecret,
        TrackingError::InvalidFormat,
        TrackingError::InvalidSignature,
        TrackingError::PayloadError("invalid JSON shape".to_owned()),
        TrackingError::Expired,
        TrackingError::NotYetValid,
        TrackingError::InvalidPolicy,
        TrackingError::ReplayDetected,
        TrackingError::ReplayStoreUnavailable,
        TrackingError::ClockUnavailable,
        TrackingError::InvalidTrackerUrl,
    ];
    for error in errors {
        let display = error.to_string();
        assert!(!display.is_empty());
        assert!(!display.contains("rullst-mail-test-key"));
    }
}
