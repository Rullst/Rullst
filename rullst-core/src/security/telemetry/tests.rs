#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn timestamps_are_absolute_rfc3339_values() {
    let timestamp = current_timestamp_str();
    assert!(chrono::DateTime::parse_from_rfc3339(&timestamp).is_ok());
    assert!(!timestamp.ends_with("s ago"));
    assert_ne!(timestamp, "Just now");
}

#[test]
fn local_events_cannot_claim_hmac_verification_or_fake_ips() {
    let store = SecurityStore::new();
    let mut event = LiveSecurityEvent::local("TEST", "local event", "attacker-controlled");
    event.verified_hmac = true;
    store.push_local_event(event);

    let events = store.live_events.lock().expect("telemetry lock");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].client_ip, "unknown");
    assert!(!events[0].verified_hmac);
}

#[test]
fn invalid_honeypot_identity_is_not_reported_as_an_active_ban() {
    let store = SecurityStore::new();
    store.record_honeypot_trap("not-a-peer-ip", "/.env");
    store.record_honeypot_observation("192.0.2.80", "/.git/config");

    assert_eq!(store.active_banned_count(), 0);
}

#[test]
fn blocked_prompt_is_not_counted_as_two_inspections() {
    let store = SecurityStore::new();
    store.record_prompt_inspected();
    store.record_prompt_injection_blocked("192.0.2.42", "ignore previous instructions");

    assert_eq!(store.prompts_inspected_count.load(Ordering::Relaxed), 1);
    assert_eq!(
        store
            .prompt_injections_blocked_count
            .load(Ordering::Relaxed),
        1
    );
    let events = store.live_events();
    assert_eq!(events[0].event_type, "AI_PROMPT_INJECTION_SHIELDED");
    assert_eq!(events[0].client_ip, "192.0.2.42");
    assert!(events[0].details.contains("ignore previous instructions"));
}

#[test]
fn rasp_interceptions_are_local_unsigned_events() {
    let store = SecurityStore::new();
    store.record_rasp_interception(false, false, true);

    let events = store.live_events.lock().expect("telemetry lock");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "RASP_PAYLOAD_INTERCEPTED");
    assert!(events[0].details.contains("body=true"));
    assert!(!events[0].verified_hmac);
}

#[test]
fn remaining_counters_increment_only_their_own_snapshot_field() {
    let store = SecurityStore::new();
    store.inc_sanitizations();
    store.inc_honeypot_traps();
    store.inc_honeypot_traps();
    store.inc_rbac_denials();
    store.inc_rbac_denials();
    store.inc_rbac_denials();
    for _ in 0..4 {
        store.inc_log_redactions();
    }
    for _ in 0..5 {
        store.inc_dlp_masked();
    }

    let snapshot = store.snapshot();
    assert_eq!(
        [
            snapshot.sanitizations,
            snapshot.honeypot_traps,
            snapshot.rbac_denials,
            snapshot.log_redactions,
            snapshot.dlp_secrets_masked,
            snapshot.prompt_injections_blocked,
            snapshot.zero_trust_mismatches,
        ],
        [1, 2, 3, 4, 5, 0, 0]
    );
    assert!(store.live_events().is_empty(), "counters record no event");
    assert_eq!(store.active_banned_count(), 0);
}

#[test]
fn a_custom_ban_ttl_sets_the_expiry() {
    let store = SecurityStore::new();
    let before = unix_timestamp_secs();
    store.record_honeypot_trap_with_ttl("192.0.2.70", "/admin.php", Duration::from_secs(90));
    let after = unix_timestamp_secs();

    let bans = store.active_bans();
    assert_eq!(bans.len(), 1);
    assert_eq!(bans[0].ip, "192.0.2.70");
    assert_eq!(bans[0].reason, "Triggered honeypot route /admin.php");
    assert!(bans[0].expires_at_unix_secs >= before + 90);
    assert!(bans[0].expires_at_unix_secs <= after + 90);
    assert!(chrono::DateTime::parse_from_rfc3339(&bans[0].timestamp_str).is_ok());
}

#[test]
fn the_global_store_is_one_instance() {
    // Only compares addresses: no other test of this binary records into it.
    assert!(std::ptr::eq(
        SecurityStore::global(),
        SecurityStore::global()
    ));
    assert!(SecurityStore::default().live_events().is_empty());
}

#[test]
fn ip_addresses_are_canonical_or_unknown() {
    assert_eq!(normalize_ip("192.0.2.1"), "192.0.2.1");
    assert_eq!(normalize_ip("2001:DB8:0:0::1"), "2001:db8::1");
    assert_eq!(normalize_ip("192.0.2.1:80"), "unknown");
    assert_eq!(normalize_ip("localhost"), "unknown");
    assert_eq!(normalize_ip(""), "unknown");
}
