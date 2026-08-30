//! Bounded HMAC session binding for application-supplied client observations.

use crate::{SecurityError, telemetry::SecurityStore};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::net::IpAddr;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Minimum key size accepted by the fingerprint HMAC boundary.
pub const MIN_FINGERPRINT_KEY_BYTES: usize = 32;
const MAX_OBSERVATION_BYTES: usize = 1_024;

/// Generates a bounded HMAC-SHA256 session fingerprint.
///
/// These observations are application supplied. This helper does not obtain or
/// validate JA3/JA4 or a TLS client certificate by itself.
pub fn try_generate_fingerprint(
    secret_key: &[u8],
    user_agent: Option<&str>,
    client_ip: Option<&str>,
    accept_language: Option<&str>,
) -> Result<String, SecurityError> {
    if secret_key.len() < MIN_FINGERPRINT_KEY_BYTES {
        return Err(SecurityError::General(format!(
            "session fingerprint key must contain at least {MIN_FINGERPRINT_KEY_BYTES} bytes"
        )));
    }
    let user_agent = bounded_observation("user-agent", user_agent.unwrap_or("unknown"))?;
    let accept_language =
        bounded_observation("accept-language", accept_language.unwrap_or("unknown"))?;
    let subnet = match client_ip {
        Some(client_ip) => normalized_subnet(client_ip)?,
        None => "unknown".to_string(),
    };
    let payload = format!("UA:{user_agent}|IP:{subnet}|LANG:{accept_language}");
    let mut mac = HmacSha256::new_from_slice(secret_key).map_err(|_| {
        SecurityError::General("session fingerprint HMAC key is invalid".to_string())
    })?;
    mac.update(payload.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Compatibility helper returning an empty, unverifiable value on invalid configuration.
///
/// Prefer [`try_generate_fingerprint`] during session creation so startup or
/// authentication can propagate the typed configuration error.
pub fn generate_fingerprint(
    secret_key: &[u8],
    user_agent: Option<&str>,
    client_ip: Option<&str>,
    accept_language: Option<&str>,
) -> String {
    try_generate_fingerprint(secret_key, user_agent, client_ip, accept_language).unwrap_or_default()
}

/// Performs constant-time verification and always rejects invalid configuration.
pub fn verify_fingerprint(
    expected_fp: &str,
    secret_key: &[u8],
    user_agent: Option<&str>,
    client_ip: Option<&str>,
    accept_language: Option<&str>,
) -> bool {
    let Ok(current_fp) =
        try_generate_fingerprint(secret_key, user_agent, client_ip, accept_language)
    else {
        SecurityStore::global().inc_zero_trust_mismatches();
        return false;
    };
    let valid = expected_fp.len() == current_fp.len()
        && bool::from(expected_fp.as_bytes().ct_eq(current_fp.as_bytes()));
    if !valid {
        SecurityStore::global().inc_zero_trust_mismatches();
    }
    valid
}

fn bounded_observation<'a>(label: &str, value: &'a str) -> Result<&'a str, SecurityError> {
    if value.len() > MAX_OBSERVATION_BYTES || value.chars().any(char::is_control) {
        return Err(SecurityError::General(format!(
            "session fingerprint {label} observation is invalid or too large"
        )));
    }
    Ok(value)
}

fn normalized_subnet(value: &str) -> Result<String, SecurityError> {
    let address = value.parse::<IpAddr>().map_err(|_| {
        SecurityError::General("session fingerprint client IP is invalid".to_string())
    })?;
    Ok(match address {
        IpAddr::V4(address) => {
            let octets = address.octets();
            format!("{}.{}.{}.0/24", octets[0], octets[1], octets[2])
        }
        IpAddr::V6(address) => {
            let segments = address.segments();
            format!(
                "{:x}:{:x}:{:x}:{:x}::/64",
                segments[0], segments[1], segments[2], segments[3]
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"0123456789abcdef0123456789abcdef";

    #[test]
    fn matching_observations_and_subnets_verify() {
        let first = try_generate_fingerprint(
            KEY,
            Some("Mozilla/5.0"),
            Some("192.168.1.10"),
            Some("en-US"),
        )
        .expect("valid fingerprint");
        let same_subnet = generate_fingerprint(
            KEY,
            Some("Mozilla/5.0"),
            Some("192.168.1.55"),
            Some("en-US"),
        );

        assert_eq!(first, same_subnet);
        assert!(verify_fingerprint(
            &first,
            KEY,
            Some("Mozilla/5.0"),
            Some("192.168.1.88"),
            Some("en-US")
        ));
    }

    #[test]
    fn weak_keys_invalid_ips_and_empty_compatibility_values_fail_closed() {
        assert!(try_generate_fingerprint(b"weak", None, None, None).is_err());
        assert!(try_generate_fingerprint(KEY, None, Some("not-an-ip"), None).is_err());
        assert_eq!(generate_fingerprint(b"weak", None, None, None), "");
        assert!(!verify_fingerprint("", b"weak", None, None, None));
    }

    #[test]
    fn changed_observations_and_ipv6_subnets_do_not_match() {
        let ipv4 = generate_fingerprint(
            KEY,
            Some("Mozilla/5.0"),
            Some("192.168.1.10"),
            Some("en-US"),
        );
        assert!(!verify_fingerprint(
            &ipv4,
            KEY,
            Some("Curl/8"),
            Some("192.168.1.10"),
            Some("en-US")
        ));

        let ipv6 =
            generate_fingerprint(KEY, Some("client"), Some("2001:db8:1:2::1"), Some("pt-BR"));
        assert!(verify_fingerprint(
            &ipv6,
            KEY,
            Some("client"),
            Some("2001:db8:1:2::99"),
            Some("pt-BR")
        ));
        assert!(!verify_fingerprint(
            &ipv6,
            KEY,
            Some("client"),
            Some("2001:db8:1:3::1"),
            Some("pt-BR")
        ));
    }
}
