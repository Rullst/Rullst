#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_mail::tracking::TrackingEngine;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let tracker_url = "https://app.rullst.dev/track";
        let secret = b"test_fuzz_hmac_secret_key_123456";
        let timestamp = 1700000000;

        let _ = TrackingEngine::inject_open_pixel(s, tracker_url);
        let _ = TrackingEngine::rewrite_links(s, tracker_url, secret, s, timestamp);

        let token = TrackingEngine::generate_open_token(secret, s, "campaign-1", timestamp);
        let _ = TrackingEngine::verify_open_token(secret, &token);

        let click_token = TrackingEngine::generate_click_token(secret, s, s, timestamp);
        let _ = TrackingEngine::verify_click_token(secret, &click_token);
    }
});
