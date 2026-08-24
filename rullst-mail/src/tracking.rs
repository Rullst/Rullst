//! Time-bounded, HMAC-authenticated email open and click tracking tokens.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

const TOKEN_VERSION: &str = "v2";
const OPEN_PURPOSE: &[u8] = b"open";
const CLICK_PURPOSE: &[u8] = b"click";
const ALLOWED_CLOCK_SKEW_SECS: u64 = 300;

/// Minimum accepted tracking HMAC key length in bytes.
pub const MIN_TRACKING_SECRET_LEN: usize = 32;
/// Default validity window used by the compatibility verification methods.
pub const DEFAULT_TRACKING_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

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

/// Errors occurring during token generation or verification.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TrackingError {
    /// The HMAC secret is shorter than 32 bytes or has trivially low diversity.
    WeakSecret,
    /// Malformed base64 or token payload format.
    InvalidFormat,
    /// Cryptographic HMAC signature mismatch.
    InvalidSignature,
    /// Deserialization of the token payload failed.
    PayloadError(String),
    /// The token timestamp is older than the configured TTL.
    Expired,
    /// The token timestamp is too far in the future.
    NotYetValid,
    /// A zero-second TTL or zero replay-cache capacity was configured.
    InvalidPolicy,
    /// A one-time verification policy already consumed this token.
    ReplayDetected,
    /// The replay cache cannot be accessed or is at capacity.
    ReplayStoreUnavailable,
    /// The system clock could not be converted to Unix time.
    ClockUnavailable,
    /// A tracking endpoint URL is invalid or not HTTP(S).
    InvalidTrackerUrl,
}

impl std::fmt::Display for TrackingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WeakSecret => write!(f, "tracking HMAC secret must be at least 32 strong bytes"),
            Self::InvalidFormat => write!(f, "invalid tracking token format"),
            Self::InvalidSignature => write!(f, "HMAC signature verification failed"),
            Self::PayloadError(message) => write!(f, "failed to decode payload: {message}"),
            Self::Expired => write!(f, "tracking token expired"),
            Self::NotYetValid => write!(f, "tracking token timestamp is in the future"),
            Self::InvalidPolicy => write!(f, "tracking TTL and replay capacity must be non-zero"),
            Self::ReplayDetected => write!(f, "tracking token replay detected"),
            Self::ReplayStoreUnavailable => write!(f, "tracking replay store unavailable"),
            Self::ClockUnavailable => write!(f, "system clock is before the Unix epoch"),
            Self::InvalidTrackerUrl => write!(f, "tracker URL must be an absolute HTTP(S) URL"),
        }
    }
}

impl std::error::Error for TrackingError {}

/// Stateless token generation and verification helpers.
pub struct TrackingEngine;

impl TrackingEngine {
    /// Generates a versioned, purpose-bound open token.
    pub fn try_generate_open_token(
        secret: &[u8],
        email: impl Into<String>,
        campaign_id: impl Into<String>,
        timestamp: u64,
    ) -> Result<String, TrackingError> {
        sign_event(
            secret,
            OPEN_PURPOSE,
            &OpenEvent {
                email: email.into(),
                campaign_id: campaign_id.into(),
                timestamp,
            },
        )
    }

    /// Legacy infallible generator. Weak keys fail closed by returning an empty token.
    #[deprecated(since = "12.0.0", note = "use TrackingEngine::try_generate_open_token")]
    // The zero-panic policy intentionally avoids unwrap-family calls in production code.
    #[allow(clippy::manual_unwrap_or_default)]
    pub fn generate_open_token(
        secret: &[u8],
        email: &str,
        campaign_id: &str,
        timestamp: u64,
    ) -> String {
        match Self::try_generate_open_token(secret, email, campaign_id, timestamp) {
            Ok(token) => token,
            Err(_) => String::new(),
        }
    }

    /// Verifies an open token using the default TTL and current system time.
    pub fn verify_open_token(secret: &[u8], token: &str) -> Result<OpenEvent, TrackingError> {
        Self::verify_open_token_at(secret, token, unix_now()?, DEFAULT_TRACKING_TTL)
    }

    /// Verifies an open token at a caller-supplied time for deterministic handlers/tests.
    pub fn verify_open_token_at(
        secret: &[u8],
        token: &str,
        now: u64,
        ttl: Duration,
    ) -> Result<OpenEvent, TrackingError> {
        let event: OpenEvent = verify_event(secret, OPEN_PURPOSE, token)?;
        validate_freshness(event.timestamp, now, ttl)?;
        Ok(event)
    }

    /// Generates a versioned, purpose-bound click token.
    pub fn try_generate_click_token(
        secret: &[u8],
        email: impl Into<String>,
        target_url: impl Into<String>,
        timestamp: u64,
    ) -> Result<String, TrackingError> {
        sign_event(
            secret,
            CLICK_PURPOSE,
            &ClickEvent {
                email: email.into(),
                target_url: target_url.into(),
                timestamp,
            },
        )
    }

    /// Legacy infallible generator. Weak keys fail closed by returning an empty token.
    #[deprecated(
        since = "12.0.0",
        note = "use TrackingEngine::try_generate_click_token"
    )]
    // The zero-panic policy intentionally avoids unwrap-family calls in production code.
    #[allow(clippy::manual_unwrap_or_default)]
    pub fn generate_click_token(
        secret: &[u8],
        email: &str,
        target_url: &str,
        timestamp: u64,
    ) -> String {
        match Self::try_generate_click_token(secret, email, target_url, timestamp) {
            Ok(token) => token,
            Err(_) => String::new(),
        }
    }

    /// Verifies a click token using the default TTL and current system time.
    pub fn verify_click_token(secret: &[u8], token: &str) -> Result<ClickEvent, TrackingError> {
        Self::verify_click_token_at(secret, token, unix_now()?, DEFAULT_TRACKING_TTL)
    }

    /// Verifies a click token at a caller-supplied time for deterministic handlers/tests.
    pub fn verify_click_token_at(
        secret: &[u8],
        token: &str,
        now: u64,
        ttl: Duration,
    ) -> Result<ClickEvent, TrackingError> {
        let event: ClickEvent = verify_event(secret, CLICK_PURPOSE, token)?;
        validate_freshness(event.timestamp, now, ttl)?;
        Ok(event)
    }

    /// Validates an endpoint and injects a transparent tracking pixel.
    pub fn try_inject_open_pixel(html: &str, tracker_url: &str) -> Result<String, TrackingError> {
        validate_tracker_url(tracker_url)?;
        let escaped_url = escape_html_attribute(tracker_url);
        let img_tag = format!(
            r#"<img src="{escaped_url}" width="1" height="1" style="display:none;width:1px;height:1px;border:0;" alt="" />"#
        );
        if let Some(position) = html.to_ascii_lowercase().rfind("</body>") {
            let mut result = String::with_capacity(html.len() + img_tag.len());
            result.push_str(&html[..position]);
            result.push_str(&img_tag);
            result.push_str(&html[position..]);
            Ok(result)
        } else {
            Ok(format!("{html}{img_tag}"))
        }
    }

    /// Legacy injection helper. Invalid tracker URLs leave HTML unchanged.
    #[deprecated(since = "12.0.0", note = "use TrackingEngine::try_inject_open_pixel")]
    pub fn inject_open_pixel(html: &str, tracker_url: &str) -> String {
        match Self::try_inject_open_pixel(html, tracker_url) {
            Ok(result) => result,
            Err(_) => html.to_string(),
        }
    }

    /// Rewrites absolute HTTP(S) links through a validated tracker endpoint.
    pub fn try_rewrite_links(
        html: &str,
        base_tracker_url: &str,
        secret: &[u8],
        email: &str,
        timestamp: u64,
    ) -> Result<String, TrackingError> {
        validate_secret(secret)?;
        validate_tracker_url(base_tracker_url)?;
        let base_clean = base_tracker_url.trim_end_matches('/');
        let mut output = String::with_capacity(html.len() + 256);
        let mut last_index = 0;
        let pattern = "href=\"";

        while let Some(start) = html[last_index..].find(pattern) {
            let href_start = last_index + start + pattern.len();
            output.push_str(&html[last_index..href_start]);
            let Some(end) = html[href_start..].find('"') else {
                break;
            };
            let url_end = href_start + end;
            let target_url = &html[href_start..url_end];
            if target_url.starts_with("http://") || target_url.starts_with("https://") {
                let token = Self::try_generate_click_token(secret, email, target_url, timestamp)?;
                output.push_str(&format!("{base_clean}/track/click/{token}"));
            } else {
                output.push_str(target_url);
            }
            last_index = url_end;
        }
        output.push_str(&html[last_index..]);
        Ok(output)
    }

    /// Legacy link rewriter. Invalid secrets or tracker URLs leave HTML unchanged.
    #[deprecated(since = "12.0.0", note = "use TrackingEngine::try_rewrite_links")]
    pub fn rewrite_links(
        html: &str,
        base_tracker_url: &str,
        secret: &[u8],
        email: &str,
        timestamp: u64,
    ) -> String {
        match Self::try_rewrite_links(html, base_tracker_url, secret, email, timestamp) {
            Ok(result) => result,
            Err(_) => html.to_string(),
        }
    }
}

/// Stateful verifier for endpoints that require one-time token consumption.
pub struct TrackingVerifier {
    ttl: Duration,
    max_entries: usize,
    seen: Mutex<HashMap<[u8; 32], u64>>,
}

impl TrackingVerifier {
    /// Creates a bounded replay-aware verifier.
    pub fn new(ttl: Duration, max_entries: usize) -> Result<Self, TrackingError> {
        if ttl.is_zero() || max_entries == 0 {
            return Err(TrackingError::InvalidPolicy);
        }
        Ok(Self {
            ttl,
            max_entries,
            seen: Mutex::new(HashMap::new()),
        })
    }

    /// Verifies and consumes an open token exactly once during its validity window.
    pub fn verify_open_once(
        &self,
        secret: &[u8],
        token: &str,
        now: u64,
    ) -> Result<OpenEvent, TrackingError> {
        let event = TrackingEngine::verify_open_token_at(secret, token, now, self.ttl)?;
        self.consume(token, event.timestamp, now)?;
        Ok(event)
    }

    /// Verifies and consumes a click token exactly once during its validity window.
    pub fn verify_click_once(
        &self,
        secret: &[u8],
        token: &str,
        now: u64,
    ) -> Result<ClickEvent, TrackingError> {
        let event = TrackingEngine::verify_click_token_at(secret, token, now, self.ttl)?;
        self.consume(token, event.timestamp, now)?;
        Ok(event)
    }

    fn consume(&self, token: &str, issued_at: u64, now: u64) -> Result<(), TrackingError> {
        let key: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        let mut seen = self
            .seen
            .lock()
            .map_err(|_| TrackingError::ReplayStoreUnavailable)?;
        seen.retain(|_, expires_at| *expires_at >= now);
        if seen.contains_key(&key) {
            return Err(TrackingError::ReplayDetected);
        }
        if seen.len() >= self.max_entries {
            return Err(TrackingError::ReplayStoreUnavailable);
        }
        seen.insert(key, issued_at.saturating_add(self.ttl.as_secs()));
        Ok(())
    }
}

fn sign_event<T: serde::Serialize>(
    secret: &[u8],
    purpose: &[u8],
    event: &T,
) -> Result<String, TrackingError> {
    validate_secret(secret)?;
    let json = serde_json::to_vec(event)
        .map_err(|error| TrackingError::PayloadError(error.to_string()))?;
    let payload = URL_SAFE_NO_PAD.encode(json);
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| TrackingError::WeakSecret)?;
    update_mac(&mut mac, purpose, &payload);
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("{TOKEN_VERSION}.{payload}.{signature}"))
}

fn verify_event<T: serde::de::DeserializeOwned>(
    secret: &[u8],
    purpose: &[u8],
    token: &str,
) -> Result<T, TrackingError> {
    validate_secret(secret)?;
    let mut parts = token.split('.');
    let version = parts.next().ok_or(TrackingError::InvalidFormat)?;
    let payload = parts.next().ok_or(TrackingError::InvalidFormat)?;
    let signature = parts.next().ok_or(TrackingError::InvalidFormat)?;
    if version != TOKEN_VERSION || parts.next().is_some() {
        return Err(TrackingError::InvalidFormat);
    }
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| TrackingError::InvalidFormat)?;
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| TrackingError::WeakSecret)?;
    update_mac(&mut mac, purpose, payload);
    mac.verify_slice(&signature)
        .map_err(|_| TrackingError::InvalidSignature)?;
    let json = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| TrackingError::InvalidFormat)?;
    serde_json::from_slice(&json).map_err(|error| TrackingError::PayloadError(error.to_string()))
}

fn update_mac(mac: &mut HmacSha256, purpose: &[u8], payload: &str) {
    mac.update(b"rullst-mail:tracking:v2\0");
    mac.update(purpose);
    mac.update(b"\0");
    mac.update(payload.as_bytes());
}

fn validate_secret(secret: &[u8]) -> Result<(), TrackingError> {
    if secret.len() < MIN_TRACKING_SECRET_LEN {
        return Err(TrackingError::WeakSecret);
    }
    let mut observed = [false; 256];
    let mut unique = 0usize;
    for byte in secret {
        let slot = &mut observed[*byte as usize];
        if !*slot {
            *slot = true;
            unique += 1;
        }
    }
    if unique < 8 {
        Err(TrackingError::WeakSecret)
    } else {
        Ok(())
    }
}

fn validate_freshness(timestamp: u64, now: u64, ttl: Duration) -> Result<(), TrackingError> {
    if ttl.is_zero() {
        return Err(TrackingError::InvalidPolicy);
    }
    if timestamp > now.saturating_add(ALLOWED_CLOCK_SKEW_SECS) {
        return Err(TrackingError::NotYetValid);
    }
    if now.saturating_sub(timestamp) > ttl.as_secs() {
        return Err(TrackingError::Expired);
    }
    Ok(())
}

fn validate_tracker_url(value: &str) -> Result<(), TrackingError> {
    let url = reqwest::Url::parse(value).map_err(|_| TrackingError::InvalidTrackerUrl)?;
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_some() {
        Ok(())
    } else {
        Err(TrackingError::InvalidTrackerUrl)
    }
}

fn unix_now() -> Result<u64, TrackingError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| TrackingError::ClockUnavailable)
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "tracking_tests.rs"]
mod tests;
