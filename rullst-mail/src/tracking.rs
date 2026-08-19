//! Zero-Cookie, Privacy-Preserving Email Open & Click Tracking Engine.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 1x1 Transparent GIF byte slice (43 bytes).
pub const PIXEL_1X1_GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\xff\xff\xff\x00\x00\x00!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;";

/// Open event payload decoded from an open tracking token.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenEvent {
    /// Recipient email address.
    pub email: String,
    /// Campaign or message identifier.
    pub campaign_id: String,
    /// Unix timestamp in seconds when the email was dispatched.
    pub timestamp: u64,
}

/// Click event payload decoded from a click tracking token.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClickEvent {
    /// Recipient email address.
    pub email: String,
    /// Target destination URL.
    pub target_url: String,
    /// Unix timestamp in seconds when the link was generated.
    pub timestamp: u64,
}

/// Errors occurring during token generation or signature verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackingError {
    /// Malformed base64 or token payload format.
    InvalidFormat,
    /// Cryptographic HMAC signature mismatch.
    InvalidSignature,
    /// Deserialization of the token payload failed.
    PayloadError(String),
}

impl std::fmt::Display for TrackingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackingError::InvalidFormat => write!(f, "Invalid tracking token format"),
            TrackingError::InvalidSignature => write!(f, "HMAC signature verification failed"),
            TrackingError::PayloadError(msg) => write!(f, "Failed to decode payload: {}", msg),
        }
    }
}

impl std::error::Error for TrackingError {}

/// Privacy-preserving tracking engine for emails without third-party tracking cookies.
pub struct TrackingEngine;

impl TrackingEngine {
    /// Generates a signed HMAC-SHA256 URL-safe token for tracking email opens.
    pub fn generate_open_token(
        secret: &[u8],
        email: &str,
        campaign_id: &str,
        timestamp: u64,
    ) -> String {
        let event = OpenEvent {
            email: email.to_string(),
            campaign_id: campaign_id.to_string(),
            timestamp,
        };

        let json_bytes = serde_json::to_vec(&event).unwrap_or_default();
        let payload_b64 = URL_SAFE_NO_PAD.encode(&json_bytes);

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(payload_b64.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        format!("{}.{}", payload_b64, signature)
    }

    /// Verifies and decodes an open tracking token.
    pub fn verify_open_token(secret: &[u8], token: &str) -> Result<OpenEvent, TrackingError> {
        let (payload_b64, sig_b64) = token.split_once('.').ok_or(TrackingError::InvalidFormat)?;

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(payload_b64.as_bytes());
        let expected_sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        if expected_sig != sig_b64 {
            return Err(TrackingError::InvalidSignature);
        }

        let json_bytes = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| TrackingError::InvalidFormat)?;

        serde_json::from_slice(&json_bytes).map_err(|e| TrackingError::PayloadError(e.to_string()))
    }

    /// Injects a transparent 1x1 tracking pixel `<img>` tag right before `</body>` or at the end of the HTML body.
    pub fn inject_open_pixel(html: &str, tracker_url: &str) -> String {
        let img_tag = format!(
            r#"<img src="{}" width="1" height="1" style="display:none;width:1px;height:1px;border:0;" alt="" />"#,
            tracker_url
        );

        if let Some(pos) = html.to_lowercase().rfind("</body>") {
            let mut result = String::with_capacity(html.len() + img_tag.len());
            result.push_str(&html[..pos]);
            result.push_str(&img_tag);
            result.push_str(&html[pos..]);
            result
        } else {
            format!("{}{}", html, img_tag)
        }
    }

    /// Generates a signed HMAC-SHA256 URL-safe token for tracking link clicks.
    pub fn generate_click_token(
        secret: &[u8],
        email: &str,
        target_url: &str,
        timestamp: u64,
    ) -> String {
        let event = ClickEvent {
            email: email.to_string(),
            target_url: target_url.to_string(),
            timestamp,
        };

        let json_bytes = serde_json::to_vec(&event).unwrap_or_default();
        let payload_b64 = URL_SAFE_NO_PAD.encode(&json_bytes);

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(payload_b64.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        format!("{}.{}", payload_b64, signature)
    }

    /// Verifies and decodes a click tracking token to retrieve the original destination URL.
    pub fn verify_click_token(secret: &[u8], token: &str) -> Result<ClickEvent, TrackingError> {
        let (payload_b64, sig_b64) = token.split_once('.').ok_or(TrackingError::InvalidFormat)?;

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(payload_b64.as_bytes());
        let expected_sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        if expected_sig != sig_b64 {
            return Err(TrackingError::InvalidSignature);
        }

        let json_bytes = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| TrackingError::InvalidFormat)?;

        serde_json::from_slice(&json_bytes).map_err(|e| TrackingError::PayloadError(e.to_string()))
    }

    /// Rewrites all `href="http..."` links in an HTML email to route through a tracking endpoint.
    pub fn rewrite_links(
        html: &str,
        base_tracker_url: &str,
        secret: &[u8],
        email: &str,
        timestamp: u64,
    ) -> String {
        let base_clean = base_tracker_url.trim_end_matches('/');
        let mut output = String::with_capacity(html.len() + 256);
        let mut last_idx = 0;

        let pattern = "href=\"";
        while let Some(start) = html[last_idx..].find(pattern) {
            let href_start = last_idx + start + pattern.len();
            output.push_str(&html[last_idx..href_start]);

            if let Some(end) = html[href_start..].find('"') {
                let url_end = href_start + end;
                let target_url = &html[href_start..url_end];

                if target_url.starts_with("http://") || target_url.starts_with("https://") {
                    let token = Self::generate_click_token(secret, email, target_url, timestamp);
                    let tracked_url = format!("{}/track/click/{}", base_clean, token);
                    output.push_str(&tracked_url);
                } else {
                    output.push_str(target_url);
                }

                last_idx = url_end;
            } else {
                break;
            }
        }

        output.push_str(&html[last_idx..]);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_gif_length_and_header() {
        assert_eq!(PIXEL_1X1_GIF.len(), 43);
        assert!(PIXEL_1X1_GIF.starts_with(b"GIF89a"));
    }

    #[test]
    fn test_open_token_roundtrip() {
        let secret = b"my-master-secret-key";
        let token = TrackingEngine::generate_open_token(
            secret,
            "user@example.com",
            "onboarding_1",
            1700000000,
        );
        let event = TrackingEngine::verify_open_token(secret, &token).unwrap();

        assert_eq!(event.email, "user@example.com");
        assert_eq!(event.campaign_id, "onboarding_1");
        assert_eq!(event.timestamp, 1700000000);
    }

    #[test]
    fn test_open_token_invalid_signature() {
        let secret = b"my-master-secret-key";
        let token = TrackingEngine::generate_open_token(
            secret,
            "user@example.com",
            "onboarding_1",
            1700000000,
        );
        let wrong_secret = b"wrong-secret-key";

        assert_eq!(
            TrackingEngine::verify_open_token(wrong_secret, &token),
            Err(TrackingError::InvalidSignature)
        );
    }

    #[test]
    fn test_click_token_roundtrip() {
        let secret = b"my-master-secret-key";
        let token = TrackingEngine::generate_click_token(
            secret,
            "user@example.com",
            "https://rullst.dev/pricing",
            1700000000,
        );
        let event = TrackingEngine::verify_click_token(secret, &token).unwrap();

        assert_eq!(event.email, "user@example.com");
        assert_eq!(event.target_url, "https://rullst.dev/pricing");
        assert_eq!(event.timestamp, 1700000000);
    }

    #[test]
    fn test_inject_open_pixel() {
        let html = "<html><body><h1>Hello</h1></body></html>";
        let tracked = TrackingEngine::inject_open_pixel(html, "https://app.com/track/open/123");

        assert!(tracked.contains(r#"<img src="https://app.com/track/open/123""#));
        assert!(tracked.contains("</body>"));
    }

    #[test]
    fn test_rewrite_links() {
        let html = r##"<p>Visit <a href="https://example.com/login">Login</a> or <a href="#internal">Anchor</a></p>"##;
        let secret = b"secret";
        let rewritten = TrackingEngine::rewrite_links(
            html,
            "https://app.com",
            secret,
            "alice@example.com",
            1700000000,
        );

        assert!(rewritten.contains("https://app.com/track/click/"));
        assert!(rewritten.contains("href=\"#internal\""));
    }
}
