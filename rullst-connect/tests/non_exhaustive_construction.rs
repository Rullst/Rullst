//! Structs that became `#[non_exhaustive]` in 13.0.0-alpha.6 can still be
//! built, with every field set, from outside the crate.

use rullst_connect::provider::ExchangeParams;
use rullst_connect::user::DeviceAuthorizationResponse;

#[test]
fn a_device_authorization_response_is_built_with_every_field() {
    let mut response = DeviceAuthorizationResponse::new(
        "device-code",
        "WDJB-MJHT",
        "https://idp.example/device",
        900,
    );
    assert_eq!(response.device_code, "device-code");
    assert_eq!(response.user_code, "WDJB-MJHT");
    assert_eq!(response.verification_uri, "https://idp.example/device");
    assert_eq!(response.expires_in, 900);
    assert_eq!(response.verification_uri_complete, None);
    assert_eq!(response.interval, None);
    response.verification_uri_complete =
        Some("https://idp.example/device?user_code=WDJB-MJHT".into());
    response.interval = Some(5);
    assert_eq!(response.interval, Some(5));
    assert!(response.verification_uri_complete.is_some());
}

#[test]
fn exchange_params_take_an_optional_nonce_through_the_field() {
    let stored: Option<String> = Some("nonce-1".into());
    let mut params = ExchangeParams::new("code").with_code_verifier("verifier");
    params.expected_nonce = stored.as_deref();
    assert_eq!(params.auth_code, "code");
    assert_eq!(params.code_verifier, Some("verifier"));
    assert_eq!(params.expected_nonce, Some("nonce-1"));
    let absent: Option<String> = None;
    params.expected_nonce = absent.as_deref();
    assert_eq!(params.expected_nonce, None);
}
