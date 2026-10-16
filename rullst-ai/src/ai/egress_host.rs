//! Exact outbound-host allowlist normalization.

use super::egress::EgressPolicyError;
use std::net::IpAddr;

pub(super) fn normalize_allowed_host(input: &str) -> Result<String, EgressPolicyError> {
    let normalized = input
        .trim()
        .trim_matches(['[', ']'])
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if normalized.is_empty() || normalized.len() > 253 {
        return Err(EgressPolicyError::InvalidConfiguration);
    }
    if let Ok(address) = normalized.parse::<IpAddr>() {
        return if super::egress::is_public_ip(address) {
            Ok(normalized)
        } else {
            Err(EgressPolicyError::InvalidConfiguration)
        };
    }
    if normalized == "localhost"
        || normalized.ends_with(".localhost")
        || matches!(
            normalized.as_str(),
            "metadata.google.internal" | "instance-data"
        )
        || !normalized.split('.').all(valid_label)
    {
        return Err(EgressPolicyError::InvalidConfiguration);
    }
    Ok(normalized)
}

fn valid_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::normalize_allowed_host;

    #[test]
    fn allowlist_hosts_are_exact_normalized_and_public() {
        assert_eq!(
            normalize_allowed_host("API.Example.COM.").expect("valid host"),
            "api.example.com"
        );
        for invalid in [
            "",
            "localhost",
            "service.localhost",
            "127.0.0.1",
            "metadata.google.internal",
            "*.example.com",
            "-bad.example",
        ] {
            assert!(normalize_allowed_host(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn allowlist_hosts_hold_at_most_253_bytes() {
        let host = |last_label: usize| {
            let label = "a".repeat(63);
            format!("{label}.{label}.{label}.{}", "b".repeat(last_label))
        };
        assert_eq!(host(61).len(), 253);
        assert_eq!(
            normalize_allowed_host(&host(61)).expect("253-byte host"),
            host(61)
        );
        assert!(normalize_allowed_host(&host(62)).is_err());
    }
}
