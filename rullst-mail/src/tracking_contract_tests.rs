//! Bounds of tracking secrets, token freshness and tracker URLs, and the
//! anchor scan of the click-link rewriter.
use super::*;

const SECRET: &[u8] = b"rullst-mail-test-key-32-bytes-minimum-2026";
const NOW: u64 = 1_800_000_000;

fn open_token(timestamp: u64) -> String {
    TrackingEngine::try_generate_open_token(SECRET, "user@example.com", "campaign", timestamp)
        .expect("strong secret")
}

#[test]
fn compatibility_verifiers_accept_tokens_for_thirty_days() {
    assert_eq!(DEFAULT_TRACKING_TTL, Duration::from_secs(2_592_000));
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock is after the epoch")
        .as_secs();
    let day = 86_400;
    let event = TrackingEngine::verify_open_token(SECRET, &open_token(now - 29 * day))
        .expect("a 29-day-old open token is fresh");
    assert_eq!(event.campaign_id, "campaign");
    assert_eq!(
        TrackingEngine::verify_open_token(SECRET, &open_token(now - 31 * day)),
        Err(TrackingError::Expired)
    );
    let click = |timestamp| {
        TrackingEngine::try_generate_click_token(
            SECRET,
            "user@example.com",
            "https://rullst.dev",
            timestamp,
        )
        .expect("strong secret")
    };
    assert!(TrackingEngine::verify_click_token(SECRET, &click(now - 29 * day)).is_ok());
    assert_eq!(
        TrackingEngine::verify_click_token(SECRET, &click(now - 31 * day)),
        Err(TrackingError::Expired)
    );
}

#[test]
#[allow(deprecated)]
fn legacy_generators_return_verifiable_tokens() {
    let ttl = Duration::from_secs(60);
    let open = TrackingEngine::generate_open_token(SECRET, "user@example.com", "campaign", NOW);
    let event = TrackingEngine::verify_open_token_at(SECRET, &open, NOW, ttl).expect("open");
    assert_eq!(event.campaign_id, "campaign");
    let click =
        TrackingEngine::generate_click_token(SECRET, "user@example.com", "https://rullst.dev", NOW);
    let event = TrackingEngine::verify_click_token_at(SECRET, &click, NOW, ttl).expect("click");
    assert_eq!(event.target_url, "https://rullst.dev");
}

#[test]
fn a_signed_token_with_an_extra_segment_is_malformed() {
    let token = format!("{}.extra", open_token(NOW));
    assert_eq!(
        TrackingEngine::verify_open_token_at(SECRET, &token, NOW, Duration::from_secs(60)),
        Err(TrackingError::InvalidFormat)
    );
}

#[test]
fn secrets_need_32_bytes_with_eight_distinct_values() {
    let generate = |secret: &[u8]| {
        TrackingEngine::try_generate_open_token(secret, "user@example.com", "campaign", NOW)
    };
    let minimal: Vec<u8> = b"abcdefgh".iter().copied().cycle().take(32).collect();
    assert!(generate(&minimal).is_ok());
    assert_eq!(generate(&minimal[..31]), Err(TrackingError::WeakSecret));
    let seven: Vec<u8> = b"abcdefg".iter().copied().cycle().take(32).collect();
    assert_eq!(generate(&seven), Err(TrackingError::WeakSecret));
}

#[test]
fn skew_and_ttl_bounds_are_inclusive() {
    let ttl = Duration::from_secs(600);
    assert!(TrackingEngine::verify_open_token_at(SECRET, &open_token(NOW + 300), NOW, ttl).is_ok());
    assert_eq!(
        TrackingEngine::verify_open_token_at(SECRET, &open_token(NOW + 301), NOW, ttl),
        Err(TrackingError::NotYetValid)
    );
    let ttl = Duration::from_secs(60);
    assert!(TrackingEngine::verify_open_token_at(SECRET, &open_token(NOW), NOW + 60, ttl).is_ok());
    assert_eq!(
        TrackingEngine::verify_open_token_at(SECRET, &open_token(NOW), NOW + 61, ttl),
        Err(TrackingError::Expired)
    );
}

#[test]
fn tracker_urls_must_use_http_or_https() {
    let html = "<body><a href=\"https://example.com\">go</a></body>";
    for tracker in ["ftp://track.example.com/", "wss://track.example.com/"] {
        assert_eq!(
            TrackingEngine::try_inject_open_pixel(html, tracker),
            Err(TrackingError::InvalidTrackerUrl),
            "{tracker}"
        );
        assert_eq!(
            TrackingEngine::try_rewrite_links(html, tracker, SECRET, "user@example.com", NOW),
            Err(TrackingError::InvalidTrackerUrl),
            "{tracker}"
        );
    }
    assert!(TrackingEngine::try_inject_open_pixel(html, "http://track.example.com/o").is_ok());
}

fn rewrite(html: &str) -> String {
    TrackingEngine::try_rewrite_links(
        html,
        "https://track.example.com",
        SECRET,
        "user@example.com",
        NOW,
    )
    .expect("valid rewrite")
}

#[test]
fn targets_with_spaces_or_controls_stay_untracked() {
    for href in ["https://app.example/a b", "https://app.example/&#1;x"] {
        let rewritten = rewrite(&format!("<a href=\"{href}\">x</a>"));
        assert!(!rewritten.contains("/track/click/"), "{href}");
        assert!(rewritten.contains(href), "{href}");
    }
}

#[test]
fn the_anchor_state_follows_its_start_tag() {
    // A second attribute of the same start tag is still inside the anchor.
    let rewritten =
        rewrite(r#"<a data-href="https://app.example/data" href="https://app.example/d">d</a>"#);
    assert!(rewritten.contains("data-href=\"https://app.example/data\""));
    assert!(!rewritten.contains("https://app.example/d\""));
    assert_eq!(rewritten.matches("/track/click/").count(), 1);

    // Text after the start tag closes is not an attribute.
    let rewritten =
        rewrite(r#"<a href="https://app.example/a">see href="https://app.example/text"</a>"#);
    assert!(rewritten.contains("see href=\"https://app.example/text\""));
    assert_eq!(rewritten.matches("/track/click/").count(), 1);
}
