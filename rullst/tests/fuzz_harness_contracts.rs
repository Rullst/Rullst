//! Positive and negative controls for the production APIs reached by fuzzing.
#![cfg(all(feature = "auth", not(target_arch = "wasm32")))]

#[path = "../fuzz/src/lib.rs"]
mod contracts;

#[test]
fn sessions_reach_authenticated_decryption_and_tamper_rejection() {
    for input in [
        b"".as_slice(),
        b"v1.invalid",
        b"REPLACE_WITH_A_STRONG_RANDOM_KEY",
        &[0xff; 128],
    ] {
        assert_eq!(contracts::session(input), 42);
    }
}

#[test]
fn configuration_reaches_toml_and_exact_key_discovery() {
    let (key, valid) =
        contracts::config("key_id=\"public\"\napp_key=\"0123456789abcdefghijklmnopqrstuv==\"\n");
    assert!(valid);
    assert_eq!(
        key.as_deref(),
        Some(b"0123456789abcdefghijklmnopqrstuv".as_slice())
    );
    let (key, valid) = contracts::config("[unterminated");
    assert!(key.is_none());
    assert!(!valid);
}

#[test]
fn tenant_selection_requires_membership_on_every_strategy() {
    assert_eq!(contracts::tenants("tenant-a"), [true; 3]);
    assert_eq!(contracts::tenants("tenant-b"), [false; 3]);
    assert_eq!(contracts::tenants(""), [false; 3]);
    assert_eq!(contracts::tenants("../tenant-b"), [false; 3]);
}

#[test]
fn realtime_delivers_the_payload_only_to_its_own_tenant() {
    for payload in [
        "",
        "hello",
        "{\"message\":\"Olá\"}",
        "<script>untrusted payload</script>",
    ] {
        assert_eq!(
            contracts::realtime(payload).expect("bounded payload"),
            payload
        );
    }
    let boundary = "x".repeat(64 * 1024);
    assert_eq!(
        contracts::realtime(&boundary).expect("maximum payload"),
        boundary
    );
    assert!(matches!(
        contracts::realtime(&format!("{boundary}x")),
        Err(rullst::RealtimeError::PayloadTooLarge { .. })
    ));
}
