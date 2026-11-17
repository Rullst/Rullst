use super::*;

#[test]
fn identifiers_are_ascii_bounded_and_log_safe() {
    assert!(validate_identifier("source", "academy.api:v1").is_ok());
    assert!(validate_identifier("source", "").is_err());
    assert!(validate_identifier("key ID", &"x".repeat(MAX_IDENTIFIER_BYTES + 1)).is_err());
    assert!(validate_identifier("event ID", "event/escape").is_err());
    assert!(validate_identifier("event ID", "event\nheader").is_err());
    assert!(validate_identifier("event ID", "evento-á").is_err());
}

#[test]
fn cloud_endpoints_require_a_closed_https_url() {
    assert!(
        validate_endpoint("https://audit.example/v1".to_string(), EndpointScope::Cloud).is_ok()
    );
    assert!(
        validate_endpoint("http://audit.example/v1".to_string(), EndpointScope::Cloud).is_err()
    );
    assert!(validate_endpoint("ftp://audit.example/v1".to_string(), EndpointScope::Cloud).is_err());
    assert!(
        validate_endpoint(
            "https://user@audit.example/v1".to_string(),
            EndpointScope::Cloud
        )
        .is_err()
    );
    assert!(
        validate_endpoint(
            "https://audit.example/v1?q=x".to_string(),
            EndpointScope::Cloud
        )
        .is_err()
    );
    assert!(
        validate_endpoint(
            "https://audit.example/v1#x".to_string(),
            EndpointScope::Cloud
        )
        .is_err()
    );
    assert!(validate_endpoint("relative/path".to_string(), EndpointScope::Cloud).is_err());
    assert!(
        validate_endpoint(
            " https://audit.example/v1".to_string(),
            EndpointScope::Cloud
        )
        .is_err()
    );
    assert!(
        validate_endpoint(
            format!("https://audit.example/{}", "x".repeat(MAX_ENDPOINT_BYTES)),
            EndpointScope::Cloud
        )
        .is_err()
    );
}

#[test]
fn local_endpoints_require_literal_loopback_http() {
    assert!(
        validate_endpoint(
            "http://127.0.0.1:8080/audit".to_string(),
            EndpointScope::Loopback
        )
        .is_ok()
    );
    assert!(
        validate_endpoint(
            "https://[::1]:8443/audit".to_string(),
            EndpointScope::Loopback
        )
        .is_ok()
    );
    assert!(
        validate_endpoint(
            "http://localhost:8080/audit".to_string(),
            EndpointScope::Loopback
        )
        .is_err()
    );
    assert!(
        validate_endpoint(
            "http://192.0.2.1/audit".to_string(),
            EndpointScope::Loopback
        )
        .is_err()
    );
    assert!(
        validate_endpoint("ftp://127.0.0.1/audit".to_string(), EndpointScope::Loopback).is_err()
    );
}

#[test]
fn keys_distinguish_live_strength_from_explicit_mocking() {
    assert!(validate_key("", ClientMode::Mock).is_ok());
    assert!(validate_key("mock_fixture", ClientMode::Mock).is_ok());
    assert!(validate_key("short", ClientMode::Live).is_err());
    assert!(validate_key("0123456789abcdef0123456789abcdef", ClientMode::Live).is_ok());
    assert!(validate_key("bad\nkey", ClientMode::Mock).is_err());
    assert!(validate_key("chave-á", ClientMode::Mock).is_err());
    assert!(validate_key(&"x".repeat(MAX_KEY_BYTES + 1), ClientMode::Live).is_err());
}

#[test]
fn each_bound_accepts_its_exact_ceiling() {
    // 128-byte identifiers, 2,048-byte endpoints and 4,096-byte keys are the
    // largest values accepted; one more byte is refused.
    assert!(validate_identifier("source", &"s".repeat(128)).is_ok());
    assert!(validate_identifier("source", &"s".repeat(129)).is_err());

    let prefix = "https://audit.example/";
    let endpoint = format!("{prefix}{}", "e".repeat(2_048 - prefix.len()));
    assert_eq!(endpoint.len(), 2_048);
    assert_eq!(
        validate_endpoint(endpoint.clone(), EndpointScope::Cloud).as_deref(),
        Ok(endpoint.as_str())
    );
    assert!(validate_endpoint(format!("{endpoint}e"), EndpointScope::Cloud).is_err());

    assert!(validate_key(&"k".repeat(4_096), ClientMode::Live).is_ok());
    assert!(validate_key(&"k".repeat(4_097), ClientMode::Live).is_err());
}

#[test]
fn identifier_errors_name_the_rejected_field() {
    for (field, message) in [
        ("source", "audit source is invalid"),
        ("key ID", "audit key ID is invalid"),
        ("event ID", "audit event ID is invalid"),
    ] {
        assert_eq!(
            validate_identifier(field, ""),
            Err(AuditDeliveryError::InvalidConfiguration(message))
        );
    }
}
