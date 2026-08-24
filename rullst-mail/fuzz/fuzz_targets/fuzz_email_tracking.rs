#![no_main]

use libfuzzer_sys::fuzz_target;
use rullst_mail::tracking::TrackingEngine;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let tracker_url = "https://app.rullst.dev/track";
        let secret = b"test_fuzz_hmac_secret_key_123456";
        let timestamp = 1700000000;

        let _ = TrackingEngine::try_inject_open_pixel(s, tracker_url);
        let _ = TrackingEngine::try_rewrite_links(s, tracker_url, secret, s, timestamp);

        if let Ok(token) =
            TrackingEngine::try_generate_open_token(secret, s, "campaign-1", timestamp)
        {
            let _ = TrackingEngine::verify_open_token_at(
                secret,
                &token,
                timestamp,
                std::time::Duration::from_secs(60),
            );
        }

        if let Ok(click_token) = TrackingEngine::try_generate_click_token(secret, s, s, timestamp) {
            let _ = TrackingEngine::verify_click_token_at(
                secret,
                &click_token,
                timestamp,
                std::time::Duration::from_secs(60),
            );
        }
    }
});
