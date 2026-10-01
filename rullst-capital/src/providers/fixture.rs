//! Deterministic offline fixtures for the legacy checkout and portal methods.
//!
//! Fixture URLs use reserved `.invalid` hosts (RFC 6761) and carry neither the
//! customer's email address nor the application's return URL, so a deployment
//! started without credentials cannot send a browser, or personal data in a
//! query string, to a real provider domain.

pub(crate) fn checkout_url(provider: &str, plan_id: &str) -> String {
    format!(
        "https://mock.{provider}.invalid/checkout/mock_session?plan={}",
        super::url_encode(plan_id)
    )
}

pub(crate) fn portal_url(provider: &str) -> String {
    format!("https://mock.{provider}.invalid/portal/mock_portal")
}
