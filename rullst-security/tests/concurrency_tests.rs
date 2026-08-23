use rullst_security::{
    AuditChain, HoneypotState, LoginGuard, StdoutAuditLogger, is_rate_limited,
    register_deception_trap,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_login_guard_concurrent_brute_force() {
    let guard = Arc::new(LoginGuard::new());
    let mut set = JoinSet::new();

    // 50 concurrent tasks hammering the same attacker IP simultaneously
    for _ in 0..50 {
        let g = Arc::clone(&guard);
        set.spawn(async move {
            g.record_login_failure("192.168.1.100");
        });
    }

    // 50 concurrent tasks recording failures for 50 distinct individual IPs
    for i in 0..50 {
        let g = Arc::clone(&guard);
        let ip = format!("10.0.0.{}", i);
        set.spawn(async move {
            g.record_login_failure(&ip);
        });
    }

    while let Some(res) = set.join_next().await {
        res.expect("Task panicked during concurrent login failure recording");
    }

    // Attacker IP must definitely be jailed
    assert!(guard.is_jailed("192.168.1.100"));
    let penalty = guard.record_login_failure("192.168.1.100");
    assert!(
        penalty >= Duration::from_secs(5),
        "Jailed penalty delay must be >= 5s"
    );

    // Distinct IPs each only had 1 failure (max_failures is 5), so none should be jailed
    for i in 0..50 {
        let ip = format!("10.0.0.{}", i);
        assert!(
            !guard.is_jailed(&ip),
            "Single failure should not jail {}",
            ip
        );
    }
}

#[tokio::test]
async fn test_rate_limiter_high_contention() {
    let mut set = JoinSet::new();
    let limit = 25;
    let window = Duration::from_secs(60);

    for _ in 0..100 {
        set.spawn(async move { is_rate_limited("client_api_key_xyz", limit, window) });
    }

    let mut allowed_count = 0;
    let mut blocked_count = 0;

    while let Some(res) = set.join_next().await {
        let limited = res.expect("Task panicked during rate limiter evaluation");
        if limited {
            blocked_count += 1;
        } else {
            allowed_count += 1;
        }
    }

    assert_eq!(
        allowed_count, limit as usize,
        "Rate limiter must allow exactly {} requests",
        limit
    );
    assert_eq!(
        blocked_count,
        (100 - limit) as usize,
        "Rate limiter must reject excess requests under high concurrency"
    );
}

#[tokio::test]
async fn test_audit_chain_concurrent_tamper_proofing() {
    let chain = Arc::new(AuditChain::new(
        b"rullst-super-secret-hmac-key-256",
        Arc::new(StdoutAuditLogger),
    ));
    let mut set = JoinSet::new();

    for i in 0..50 {
        let c = Arc::clone(&chain);
        set.spawn(async move {
            c.record_event(
                &format!("user_{}", i),
                "UPDATE_PROFILE",
                "user_profile",
                &format!("Profile updated by user {}", i),
            )
            .await
        });
    }

    let mut records = Vec::new();
    while let Some(res) = set.join_next().await {
        let rec = res
            .expect("Audit recording task panicked")
            .expect("Audit record failed");
        assert!(!rec.hash.is_empty(), "Signature hash must not be empty");
        assert!(
            !rec.previous_hash.is_empty(),
            "Previous hash must not be empty"
        );
        records.push(rec);
    }

    assert_eq!(records.len(), 50);
}

#[tokio::test]
async fn test_honeypot_deception_concurrent_traps() {
    let trap_routes = [
        "/admin.php",
        "/.env",
        "/wp-login.php",
        "/.git/config",
        "/phpmyadmin",
        "/actuator/health",
        "/api/v1/debug",
    ];

    let state = Arc::new(HoneypotState::new(
        trap_routes.iter().map(|s| s.to_string()).collect(),
    ));
    let mut set = JoinSet::new();

    for i in 0..70 {
        let route = trap_routes[i % trap_routes.len()];
        let s = Arc::clone(&state);
        let ip = format!("185.220.101.{}", i % 10);
        set.spawn(async move {
            register_deception_trap(route);
            assert!(s.is_trap(route));
            s.ban_ip(ip);
        });
    }

    while let Some(res) = set.join_next().await {
        res.expect("Honeypot concurrent registration panicked");
    }

    assert!(state.banned_count() > 0, "Banned IPs must be recorded");
}
