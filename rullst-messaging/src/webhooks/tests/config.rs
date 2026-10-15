use super::*;

fn test_root() -> Vec<u8> {
    rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])
        .unwrap()
        .cert
        .pem()
        .into_bytes()
}

#[test]
fn destination_urls_are_bounded_at_exactly_2048_bytes() {
    let prefix = "https://receiver.example/";
    let url = format!("{prefix}{}", "a".repeat(2_048 - prefix.len()));
    assert_eq!(url.len(), 2_048);
    assert!(WebhookDestination::approved_https(&url).is_ok());
    assert_eq!(
        WebhookDestination::approved_https(format!("{url}a")).err(),
        Some(WebhookError::DestinationDenied)
    );
}

#[test]
fn credentials_in_either_userinfo_part_are_denied() {
    for url in [
        "https://user@receiver.example/events",
        "https://:secret@receiver.example/events",
    ] {
        assert_eq!(
            WebhookDestination::approved_https(url).err(),
            Some(WebhookError::DestinationDenied),
            "{url}"
        );
    }
}

#[test]
fn test_roots_need_a_loopback_https_destination_and_at_most_8_kib() {
    let root = test_root();
    let https = WebhookDestination::loopback_test("https://127.0.0.1:8443/hook").unwrap();
    let mut padded = root.clone();
    padded.resize(8_192, b'\n');
    assert!(https.clone().with_test_root_pem(padded.clone()).is_ok());
    padded.push(b'\n');
    let refused = Some(WebhookError::InvalidInput("test TLS root"));
    assert_eq!(https.with_test_root_pem(padded).err(), refused);

    let http = WebhookDestination::loopback_test("http://127.0.0.1:8080/hook").unwrap();
    assert_eq!(http.with_test_root_pem(root.clone()).err(), refused);
    let public = WebhookDestination::approved_https("https://receiver.example/hook").unwrap();
    assert_eq!(public.with_test_root_pem(root).err(), refused);
}

#[test]
fn delivery_windows_span_one_minute_to_seven_days() {
    let config = || {
        let destination = WebhookDestination::approved_https("https://receiver.example/").unwrap();
        WebhookConfig::new("windows", destination, 1).unwrap()
    };
    for millis in [60_000, 604_800_000] {
        let window = config()
            .with_delivery_window(Duration::from_millis(millis))
            .unwrap();
        assert_eq!(window.window_ms, millis as i64);
    }
    for millis in [59_999, 604_800_001] {
        assert_eq!(
            config()
                .with_delivery_window(Duration::from_millis(millis))
                .err(),
            Some(WebhookError::InvalidInput("delivery window")),
            "{millis}"
        );
    }
}

#[test]
fn debug_output_shows_only_the_loopback_policy() {
    let loopback = WebhookDestination::loopback_test("http://127.0.0.1:8080/private").unwrap();
    assert_eq!(
        format!("{loopback:?}"),
        "WebhookDestination { loopback_test: true, .. }"
    );
    let config = WebhookConfig::new("debug", loopback, 1).unwrap();
    assert_eq!(format!("{config:?}"), "WebhookConfig([REDACTED])");
}
