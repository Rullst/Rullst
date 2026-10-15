//! Address-length, dotless-domain and error-message contracts.
use super::*;

#[test]
fn recipient_addresses_hold_at_most_320_bytes() {
    let address = |length: usize| format!("{}@example.com", "a".repeat(length - 12));
    assert_eq!(address(320).len(), 320);
    assert_eq!(validate_email_deliverability(&address(320)), Ok(()));
    assert!(validate_email_deliverability(&address(321)).is_err());
}

#[test]
fn a_domain_without_a_dot_is_missing() {
    assert_eq!(
        validate_email_syntax("user@localhost"),
        Err(DeliverabilityError::MissingDomain)
    );
    assert_eq!(
        validate_email_syntax("user@"),
        Err(DeliverabilityError::MissingDomain)
    );
}

#[test]
fn errors_describe_the_failed_rule() {
    assert_eq!(
        DeliverabilityError::InvalidSyntax("Email is empty".into()).to_string(),
        "Invalid email syntax: Email is empty"
    );
    assert_eq!(
        DeliverabilityError::DisposableDomain("mailinator.com".into()).to_string(),
        "Email uses a blocked disposable provider: mailinator.com"
    );
    assert_eq!(
        DeliverabilityError::MissingDomain.to_string(),
        "Email address is missing domain part"
    );
}
