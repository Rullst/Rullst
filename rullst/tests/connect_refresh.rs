#![cfg(feature = "oauth")]

use rullst::connect::{RefreshableTokenState, prelude::SecretString};

#[test]
fn umbrella_exposes_bounded_refresh_state() {
    let state = RefreshableTokenState::try_new(
        "provider-user-1",
        SecretString::from("access-token".to_string()),
        SecretString::from("refresh-token".to_string()),
        1_800_000_000,
        3_600,
    )
    .expect("valid refresh state");

    assert_eq!(state.provider_user_id(), "provider-user-1");
    assert_eq!(state.expires_at(), 1_800_003_600);
    let debug = format!("{state:?}");
    assert!(!debug.contains("access-token"));
    assert!(!debug.contains("refresh-token"));
}
