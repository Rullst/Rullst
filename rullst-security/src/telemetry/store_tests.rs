use super::*;

fn wall_clock_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after the epoch")
        .as_secs()
}

fn ban_record(ip: &str, expires_at_unix_secs: u64) -> BannedIpRecord {
    BannedIpRecord {
        ip: ip.to_string(),
        reason: "test".to_string(),
        timestamp_str: current_timestamp_str(),
        expires_at_unix_secs,
    }
}

fn event_types(store: &SecurityStore) -> Vec<String> {
    store
        .live_events
        .lock()
        .expect("telemetry lock")
        .iter()
        .map(|event| event.event_type.clone())
        .collect()
}

#[test]
fn counter_helpers_increment_only_their_own_snapshot_field() {
    let store = SecurityStore::new();
    let helpers: [fn(&SecurityStore); 12] = [
        SecurityStore::inc_zero_trust_mismatches,
        SecurityStore::inc_schema_violations,
        SecurityStore::inc_sri_signed_assets,
        SecurityStore::inc_mfa_verifications,
        SecurityStore::inc_deception_hits,
        SecurityStore::inc_cswsh_blocks,
        SecurityStore::inc_rate_limit_blocks,
        SecurityStore::inc_siem_dispatches,
        SecurityStore::inc_login_jail_bans,
        SecurityStore::inc_secure_headers,
        SecurityStore::inc_idor_warnings,
        SecurityStore::inc_timing_guard_protected,
    ];
    for (index, helper) in helpers.iter().enumerate() {
        for _ in 0..=index {
            helper(&store);
        }
    }

    let snapshot = store.snapshot();
    assert_eq!(
        [
            snapshot.zero_trust_mismatches,
            snapshot.schema_violations,
            snapshot.sri_signed_assets,
            snapshot.mfa_verifications,
            snapshot.deception_hits,
            snapshot.cswsh_blocks,
            snapshot.rate_limit_blocks,
            snapshot.siem_dispatches,
            snapshot.login_jail_bans,
            snapshot.secure_headers_applied,
            snapshot.idor_warnings,
            snapshot.timing_guard_protected,
        ],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    );
    assert_eq!(snapshot.sanitizations, 0);
    assert_eq!(snapshot.rbac_denials, 0);
}

#[test]
fn recorders_count_and_publish_their_events() {
    let store = SecurityStore::new();
    store.record_sanitization("script tag removed");
    store.record_rbac_denial("alice", "/admin/users");
    store.record_pii_masked(3);
    store.record_pii_masked(2);

    let snapshot = store.snapshot();
    assert_eq!(snapshot.sanitizations, 1);
    assert_eq!(snapshot.rbac_denials, 1);
    assert_eq!(store.pii_masked_count.load(Ordering::Relaxed), 5);

    let events = store.live_events.lock().expect("telemetry lock");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "RBAC_ACCESS_DENIED");
    assert!(events[0].details.contains("alice"));
    assert!(events[0].details.contains("/admin/users"));
    assert_eq!(events[1].event_type, "XSS_PAYLOAD_NEUTRALIZED");
    assert_eq!(events[1].details, "script tag removed");
}

#[test]
fn honeypot_trap_bans_the_peer_for_fifteen_minutes_of_wall_clock_time() {
    let store = SecurityStore::new();
    let before = wall_clock_secs();
    store.record_honeypot_trap("192.0.2.7", "/.env");
    let after = wall_clock_secs();

    assert_eq!(store.active_banned_count(), 1);
    let record = store.banned_ips.get("192.0.2.7").expect("active ban");
    assert!(record.expires_at_unix_secs >= before + 15 * 60);
    assert!(record.expires_at_unix_secs <= after + 15 * 60);
    assert!(record.reason.contains("/.env"));
    drop(record);

    assert_eq!(store.snapshot().honeypot_traps, 1);
    let hits = store.honeypot_route_hits.get("/.env").expect("route hits");
    assert_eq!(hits.load(Ordering::Relaxed), 1);
    drop(hits);
    assert_eq!(event_types(&store), ["HONEYPOT_TRAP_TRIGGERED"]);
}

#[test]
fn ban_table_stops_growing_at_its_cap_but_refreshes_known_peers() {
    let store = SecurityStore::new();
    for index in 0..MAX_TELEMETRY_BANS {
        let ip = format!("10.0.{}.{}", index / 256, index % 256);
        store
            .banned_ips
            .insert(ip.clone(), ban_record(&ip, u64::MAX));
    }

    store.record_honeypot_trap("192.0.2.200", "/wp-admin");
    assert_eq!(store.banned_ips.len(), MAX_TELEMETRY_BANS);
    assert!(!store.banned_ips.contains_key("192.0.2.200"));

    store.record_honeypot_trap("10.0.0.5", "/wp-admin");
    assert_eq!(store.banned_ips.len(), MAX_TELEMETRY_BANS);
    let refreshed = store.banned_ips.get("10.0.0.5").expect("known peer");
    assert!(refreshed.reason.contains("/wp-admin"));
    assert_ne!(refreshed.expires_at_unix_secs, u64::MAX);
}

#[test]
fn route_hit_table_stops_growing_at_its_cap_but_counts_known_routes() {
    let store = SecurityStore::new();
    for index in 0..MAX_HONEYPOT_ROUTES {
        store
            .honeypot_route_hits
            .insert(format!("/trap/{index}"), AtomicU64::new(0));
    }

    store.record_honeypot_observation("192.0.2.9", "/trap/new");
    assert_eq!(store.honeypot_route_hits.len(), MAX_HONEYPOT_ROUTES);
    assert!(!store.honeypot_route_hits.contains_key("/trap/new"));

    store.record_honeypot_observation("192.0.2.9", "/trap/7");
    let hits = store
        .honeypot_route_hits
        .get("/trap/7")
        .expect("known route");
    assert_eq!(hits.load(Ordering::Relaxed), 1);
}

#[test]
fn pruning_drops_bans_that_expire_at_or_before_now() {
    let store = SecurityStore::new();
    let now = 1_000_000;
    store
        .banned_ips
        .insert("192.0.2.1".into(), ban_record("192.0.2.1", now - 1));
    store
        .banned_ips
        .insert("192.0.2.2".into(), ban_record("192.0.2.2", now));
    store
        .banned_ips
        .insert("192.0.2.3".into(), ban_record("192.0.2.3", now + 1));

    store.prune_expired_bans_at(now);

    let mut remaining = store
        .banned_ips
        .iter()
        .map(|entry| entry.key().clone())
        .collect::<Vec<_>>();
    remaining.sort();
    assert_eq!(remaining, ["192.0.2.3"]);
}

#[test]
fn wall_clock_pruning_drops_bans_from_the_past() {
    let store = SecurityStore::new();
    store
        .banned_ips
        .insert("192.0.2.1".into(), ban_record("192.0.2.1", 1_000));
    store
        .banned_ips
        .insert("192.0.2.2".into(), ban_record("192.0.2.2", u64::MAX));

    store.prune_expired_bans();
    assert!(!store.banned_ips.contains_key("192.0.2.1"));
    assert!(store.banned_ips.contains_key("192.0.2.2"));

    store
        .banned_ips
        .insert("192.0.2.3".into(), ban_record("192.0.2.3", 1_000));
    assert_eq!(store.active_banned_count(), 1);
}

#[test]
fn live_event_buffer_keeps_the_newest_fifty_events() {
    let store = SecurityStore::new();
    for index in 0..=MAX_LIVE_EVENTS {
        store.push_local_event(LiveSecurityEvent::local(
            "TEST",
            format!("event {index}"),
            "unknown",
        ));
    }

    let events = store.live_events.lock().expect("telemetry lock");
    assert_eq!(events.len(), MAX_LIVE_EVENTS);
    assert_eq!(events[0].details, format!("event {MAX_LIVE_EVENTS}"));
    assert_eq!(events[MAX_LIVE_EVENTS - 1].details, "event 1");
}

#[cfg(target_os = "linux")]
#[test]
fn resident_memory_reports_the_running_process() {
    let megabytes = get_real_rss_memory_mb().expect("Linux exposes VmRSS");
    assert!(megabytes > 1.0);
}
