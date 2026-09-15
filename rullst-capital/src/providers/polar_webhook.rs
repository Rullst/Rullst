//! Polar's documented Standard Webhooks envelope and secret-key migration.
use crate::CapitalError;
use base64::{Engine, engine::general_purpose::STANDARD};
use ring::hmac;
use std::collections::HashMap;

fn invalid() -> CapitalError {
    CapitalError::InvalidSignature("Invalid Polar Standard Webhooks envelope".into())
}

fn header<'a>(headers: &'a HashMap<String, String>, name: &str) -> Result<&'a str, CapitalError> {
    let mut matching = headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name));
    let (_, value) = matching.next().ok_or_else(invalid)?;
    if matching.next().is_some() {
        return Err(invalid());
    }
    Ok(value)
}

pub(super) fn verify(
    secret: &str,
    payload: &[u8],
    headers: &HashMap<String, String>,
    now: i64,
) -> Result<(), CapitalError> {
    if secret.len() > 4096 || payload.len() > 1024 * 1024 {
        return Err(invalid());
    }
    let id = header(headers, "webhook-id")?;
    let timestamp = header(headers, "webhook-timestamp")?;
    let signatures = header(headers, "webhook-signature")?;
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        || timestamp.is_empty()
        || timestamp.len() > 20
        || !timestamp.bytes().all(|byte| byte.is_ascii_digit())
        || signatures.is_empty()
        || signatures.len() > 4096
    {
        return Err(invalid());
    }
    super::ensure_fresh_timestamp(
        "polar",
        timestamp,
        now,
        super::DEFAULT_WEBHOOK_TOLERANCE,
        false,
    )?;

    // Polar's current SDK accepts the old literal UTF-8 secret key and the
    // standard whsec_ Base64 key. Both sign the same ID.timestamp.raw-body.
    let standard_key = secret
        .strip_prefix("whsec_")
        .and_then(|key| STANDARD.decode(key).ok());
    let literal_tag = sign(secret.as_bytes(), id, timestamp, payload);
    let standard_tag = standard_key
        .as_deref()
        .filter(|key| !key.is_empty())
        .map(|key| sign(key, id, timestamp, payload));
    let mut valid = false;
    let mut seen = std::collections::HashSet::new();
    for signature in signatures.split_ascii_whitespace() {
        if seen.len() >= 16 || !seen.insert(signature) {
            return Err(invalid());
        }
        let Some((version, encoded)) = signature.split_once(',') else {
            return Err(invalid());
        };
        if version != "v1" {
            continue;
        }
        let supplied = STANDARD.decode(encoded).map_err(|_| invalid())?;
        if supplied.len() != 32 {
            return Err(invalid());
        }
        use subtle::ConstantTimeEq;
        valid |= bool::from(literal_tag.as_ref().ct_eq(&supplied));
        if let Some(tag) = &standard_tag {
            valid |= bool::from(tag.as_ref().ct_eq(&supplied));
        }
    }
    if valid { Ok(()) } else { Err(invalid()) }
}

fn sign(key: &[u8], id: &str, timestamp: &str, payload: &[u8]) -> hmac::Tag {
    let mut context = hmac::Context::with_key(&hmac::Key::new(hmac::HMAC_SHA256, key));
    context.update(id.as_bytes());
    context.update(b".");
    context.update(timestamp.as_bytes());
    context.update(b".");
    context.update(payload);
    context.sign()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;
    const PAYLOAD: &[u8] = br#"{"type":"subscription.updated"}"#;
    const KEY: &[u8] = b"deterministic-polar-timestamp-contract";

    fn signed_headers(timestamp: i64) -> HashMap<String, String> {
        let id = "evt_clock_boundary";
        let timestamp = timestamp.to_string();
        // Build a genuine signature independently of the verifier's helper.
        let message = [format!("{id}.{timestamp}.").as_bytes(), PAYLOAD].concat();
        let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, KEY), &message);
        HashMap::from([
            ("webhook-id".into(), id.into()),
            ("webhook-timestamp".into(), timestamp),
            (
                "webhook-signature".into(),
                format!("v1,{}", STANDARD.encode(tag.as_ref())),
            ),
        ])
    }

    #[test]
    fn exact_clock_window_accepts_endpoints_and_rejects_adjacent_seconds() {
        let standard = format!("whsec_{}", STANDARD.encode(KEY));
        let literal = std::str::from_utf8(KEY).unwrap();
        for secret in [literal, standard.as_str()] {
            for offset in [-300, -299, 0, 299, 300] {
                assert!(
                    verify(secret, PAYLOAD, &signed_headers(NOW + offset), NOW).is_ok(),
                    "signed timestamp at offset {offset} must be accepted"
                );
            }
            for offset in [-301, 301] {
                assert!(matches!(
                    verify(secret, PAYLOAD, &signed_headers(NOW + offset), NOW),
                    Err(CapitalError::StaleWebhook(_))
                ));
            }
        }
    }

    #[test]
    fn clock_tick_moves_the_future_endpoint_into_the_accepted_window() {
        let secret = format!("whsec_{}", STANDARD.encode(KEY));
        let future = signed_headers(NOW + 301);
        assert!(matches!(
            verify(&secret, PAYLOAD, &future, NOW),
            Err(CapitalError::StaleWebhook(_))
        ));
        assert!(verify(&secret, PAYLOAD, &future, NOW + 1).is_ok());

        let past = signed_headers(NOW - 300);
        assert!(verify(&secret, PAYLOAD, &past, NOW).is_ok());
        assert!(matches!(
            verify(&secret, PAYLOAD, &past, NOW + 1),
            Err(CapitalError::StaleWebhook(_))
        ));
    }
}
