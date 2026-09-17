//! Shared bounds for additive Stripe billing operations.

pub(crate) const API_VERSION: &str = "2025-03-31.basil";

pub(crate) fn valid_reference(value: &str, prefix: &str, limit: usize) -> bool {
    value.len() > prefix.len()
        && value.len() <= limit
        && value.starts_with(prefix)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

pub(super) fn credential_mode(api_key: &str) -> Option<bool> {
    if api_key.starts_with("sk_live_") || api_key.starts_with("rk_live_") {
        Some(true)
    } else if api_key.starts_with("sk_test_") || api_key.starts_with("rk_test_") {
        Some(false)
    } else {
        None
    }
}
