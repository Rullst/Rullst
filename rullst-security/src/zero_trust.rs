use crate::telemetry::SecurityStore;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Generates a HMAC-SHA256 client session fingerprint based on User-Agent, IP subnet, and Accept-Language.
pub fn generate_fingerprint(
    secret_key: &[u8],
    user_agent: Option<&str>,
    client_ip: Option<&str>,
    accept_language: Option<&str>,
) -> String {
    let ua = user_agent.unwrap_or("unknown");
    let ip = client_ip.unwrap_or("127.0.0.1");
    // Extract IP subnet (e.g. "192.168.1" from "192.168.1.50")
    let subnet = ip.rsplit_once('.').map(|(s, _)| s).unwrap_or(ip);
    let lang = accept_language.unwrap_or("en");

    let payload = format!("UA:{}|IP:{}|LANG:{}", ua, subnet, lang);

    let mut mac = HmacSha256::new_from_slice(secret_key)
        .unwrap_or_else(|_| HmacSha256::new_from_slice(b"rullst_zero_trust_fallback_key").unwrap());
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time verification of a client session fingerprint.
pub fn verify_fingerprint(
    expected_fp: &str,
    secret_key: &[u8],
    user_agent: Option<&str>,
    client_ip: Option<&str>,
    accept_language: Option<&str>,
) -> bool {
    let current_fp = generate_fingerprint(secret_key, user_agent, client_ip, accept_language);
    let valid = expected_fp.len() == current_fp.len()
        && expected_fp
            .as_bytes()
            .ct_eq(current_fp.as_bytes())
            .into();

    if !valid {
        SecurityStore::global().inc_zero_trust_mismatches();
    }

    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_trust_fingerprint_matching() {
        let key = b"my_super_secret_key_12345";
        let fp1 = generate_fingerprint(
            key,
            Some("Mozilla/5.0"),
            Some("192.168.1.10"),
            Some("en-US"),
        );
        let fp2 = generate_fingerprint(
            key,
            Some("Mozilla/5.0"),
            Some("192.168.1.55"), // same subnet 192.168.1
            Some("en-US"),
        );

        assert_eq!(fp1, fp2);
        assert!(verify_fingerprint(
            &fp1,
            key,
            Some("Mozilla/5.0"),
            Some("192.168.1.88"),
            Some("en-US")
        ));
    }

    #[test]
    fn test_zero_trust_fingerprint_mismatch() {
        let key = b"my_super_secret_key_12345";
        let fp1 = generate_fingerprint(
            key,
            Some("Mozilla/5.0"),
            Some("192.168.1.10"),
            Some("en-US"),
        );

        // Different User-Agent
        assert!(!verify_fingerprint(
            &fp1,
            key,
            Some("Curl/7.68.0"),
            Some("192.168.1.10"),
            Some("en-US")
        ));

        // Different subnet
        assert!(!verify_fingerprint(
            &fp1,
            key,
            Some("Mozilla/5.0"),
            Some("10.0.0.5"),
            Some("en-US")
        ));
    }
}
