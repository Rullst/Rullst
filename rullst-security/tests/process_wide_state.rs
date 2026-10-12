//! Checks of process-wide state: the deception trap registry and global
//! telemetry counters. Each test owns state that no other test in this binary
//! touches, so exact before/after comparisons are deterministic here, unlike
//! in the library test binary where many tests share the global store.

use rullst_security::deception::{
    MAX_DECEPTION_TRAPS, global_deception_routes, try_register_deception_trap,
};
use rullst_security::telemetry::SecurityStore;
use rullst_security::zero_trust::{try_generate_fingerprint, verify_fingerprint};
use rullst_security::{RbacGuard, UserContext};

#[test]
fn a_full_deception_registry_rejects_new_traps_but_accepts_known_ones() {
    let mut index = 0;
    while global_deception_routes().len() < MAX_DECEPTION_TRAPS {
        try_register_deception_trap(&format!("/capacity-trap-{index}"))
            .expect("registry below capacity");
        index += 1;
    }
    assert!(try_register_deception_trap("/one-trap-too-many").is_err());
    assert!(!global_deception_routes().contains("/one-trap-too-many"));
    assert!(try_register_deception_trap("/.env").is_ok());
    assert!(try_register_deception_trap("/capacity-trap-0").is_ok());
    assert_eq!(global_deception_routes().len(), MAX_DECEPTION_TRAPS);
}

#[test]
fn only_a_fingerprint_mismatch_increments_the_mismatch_counter() {
    let key = b"0123456789abcdef0123456789abcdef";
    let (agent, address, language) = (Some("Mozilla/5.0"), Some("192.0.2.1"), Some("en-GB"));
    let fingerprint = try_generate_fingerprint(key, agent, address, language).expect("fingerprint");
    let mismatches = || SecurityStore::global().snapshot().zero_trust_mismatches;

    let before = mismatches();
    assert!(verify_fingerprint(
        &fingerprint,
        key,
        agent,
        address,
        language
    ));
    assert_eq!(mismatches(), before);
    assert!(!verify_fingerprint(
        &fingerprint,
        key,
        Some("curl/8.0"),
        address,
        language
    ));
    assert_eq!(mismatches(), before + 1);
}

#[test]
fn a_tenant_denial_is_counted_once() {
    let denials = || SecurityStore::global().snapshot().rbac_denials;
    let foreign_admin = UserContext::new("tenant-probe", vec!["admin".to_string()])
        .try_with_tenant_id("school-2")
        .expect("tenant context");

    let before = denials();
    assert!(RbacGuard::authorize_tenant(&foreign_admin, "school-1").is_err());
    assert_eq!(denials(), before + 1);
}
