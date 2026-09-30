use super::*;
use axum::{Router, routing::get};
use std::thread;
use tower::ServiceExt;

#[test]
fn traps_use_exact_paths_and_bans_expire() {
    let state =
        HoneypotState::try_with_limits(vec!["/.env".to_string()], Duration::from_millis(10), 2)
            .expect("valid honeypot state");

    assert!(state.is_trap("/.env"));
    assert!(!state.is_trap("/download/.env"));
    assert!(!state.is_trap("/.environment"));

    state.ban_ip("192.0.2.1".to_string());
    assert!(state.is_banned("192.0.2.1"));
    thread::sleep(Duration::from_millis(20));
    assert!(!state.is_banned("192.0.2.1"));
}

#[test]
fn ban_cardinality_is_bounded_and_invalid_ips_are_ignored() {
    let state =
        HoneypotState::try_with_limits(vec!["/.env".to_string()], Duration::from_secs(60), 2)
            .expect("valid honeypot state");

    state.ban_ip("not-an-ip".to_string());
    state.ban_ip("192.0.2.1".to_string());
    state.ban_ip("192.0.2.2".to_string());
    state.ban_ip("192.0.2.3".to_string());
    assert_eq!(state.banned_count(), 2);
}

#[test]
fn request_lookup_touches_only_the_requested_peer() {
    let state =
        HoneypotState::try_with_limits(vec!["/.env".to_string()], Duration::from_secs(60), 4)
            .expect("valid honeypot state");
    state.ban_ip("192.0.2.3".to_string());
    let expired = Instant::now() - Duration::from_secs(1);
    {
        let mut bans = state.banned_ips.lock().unwrap();
        for index in 1..=2 {
            bans.insert(IpAddr::from([192, 0, 2, index]), expired, state.max_bans);
        }
    }

    assert!(state.is_banned("192.0.2.3"));
    assert!(!state.is_banned("198.51.100.1"));
    assert_eq!(state.banned_ips.lock().unwrap().len(), 3);

    assert!(!state.is_banned("192.0.2.1"));
    assert_eq!(state.banned_ips.lock().unwrap().len(), 2);

    assert_eq!(state.banned_count(), 1);
}

#[test]
fn full_ban_list_evicts_the_soonest_expiring_ban_and_refreshes_existing_peers() {
    let state =
        HoneypotState::try_with_limits(vec!["/.env".to_string()], Duration::from_secs(60), 2)
            .expect("valid honeypot state");
    let now = Instant::now();
    {
        let mut bans = state.banned_ips.lock().unwrap();
        let first = IpAddr::from([192, 0, 2, 1]);
        bans.insert(first, now + Duration::from_secs(10), state.max_bans);
        bans.insert(
            IpAddr::from([192, 0, 2, 2]),
            now + Duration::from_secs(20),
            state.max_bans,
        );
        // Refreshing an existing peer moves it behind the other ban.
        bans.insert(first, now + Duration::from_secs(30), state.max_bans);
    }
    assert_eq!(state.banned_count(), 2);

    state.ban_ip("192.0.2.3".to_string());
    assert_eq!(state.banned_count(), 2);
    assert!(state.is_banned("192.0.2.1"));
    assert!(!state.is_banned("192.0.2.2"));
    assert!(state.is_banned("192.0.2.3"));
}

#[test]
fn strict_configuration_rejects_unsafe_or_unbounded_values() {
    let valid_path = vec!["/.env".to_string()];
    assert!(HoneypotState::try_with_limits(valid_path.clone(), Duration::ZERO, 1).is_err());
    assert!(HoneypotState::try_with_limits(valid_path, Duration::from_secs(1), 0).is_err());
    assert!(
        HoneypotState::try_with_limits(
            vec!["relative/path".to_string()],
            Duration::from_secs(1),
            1,
        )
        .is_err()
    );
    assert!(HoneypotState::try_with_limits(Vec::new(), Duration::from_secs(1), 1).is_err());

    let too_many_paths = (0..=MAX_HONEYPOT_TRAP_PATHS)
        .map(|index| format!("/trap-{index}"))
        .collect();
    assert!(HoneypotState::try_with_limits(too_many_paths, Duration::from_secs(1), 1).is_err());
}

#[test]
fn compatibility_constructor_filters_invalid_and_duplicate_paths() {
    let state = HoneypotState::new(vec![
        "/Trap".to_string(),
        "/trap".to_string(),
        "relative".to_string(),
        "/bad?query=true".to_string(),
    ]);
    assert!(state.is_trap("/trap"));
    assert_eq!(state.trap_paths.len(), 1);
}

#[tokio::test]
async fn middleware_ignores_forwarded_identity_and_uses_socket_peer() {
    let state = HoneypotState::new(vec!["/.env".to_string()]);
    let app = Router::new()
        .route("/{*path}", get(|| async { StatusCode::OK }))
        .layer(HoneypotLayer::new(state.clone()));

    let forged = Request::builder()
        .uri("/.env")
        .header("x-forwarded-for", "198.51.100.99")
        .body(Body::empty())
        .expect("valid request");
    let response = app.clone().oneshot(forged).await.expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!state.is_banned("198.51.100.99"));

    let mut verified = Request::builder()
        .uri("/.env")
        .header("x-forwarded-for", "198.51.100.99")
        .body(Body::empty())
        .expect("valid request");
    verified.extensions_mut().insert(ConnectInfo(
        "192.0.2.25:443"
            .parse::<SocketAddr>()
            .expect("valid socket address"),
    ));
    let response = app.clone().oneshot(verified).await.expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(state.is_banned("192.0.2.25"));
    assert!(!state.is_banned("198.51.100.99"));

    let mut banned_peer = Request::builder()
        .uri("/safe")
        .body(Body::empty())
        .expect("valid request");
    banned_peer.extensions_mut().insert(ConnectInfo(
        "192.0.2.25:8443"
            .parse::<SocketAddr>()
            .expect("valid socket address"),
    ));
    let response = app.clone().oneshot(banned_peer).await.expect("response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let lookalike = Request::builder()
        .uri("/download/.env")
        .body(Body::empty())
        .expect("valid request");
    let response = app.oneshot(lookalike).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

fn trap_request(peer: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder().uri("/.env");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::empty()).expect("valid request");
    request.extensions_mut().insert(ConnectInfo(
        peer.parse::<SocketAddr>().expect("valid socket address"),
    ));
    request
}

#[tokio::test]
async fn page_initiated_trap_loads_are_refused_without_banning_the_visitor() {
    let state = HoneypotState::new(vec!["/.env".to_string()]);
    let app = Router::new()
        .route("/{*path}", get(|| async { StatusCode::OK }))
        .layer(HoneypotLayer::new(state.clone()));

    let lures: [(&str, &[(&str, &str)]); 5] = [
        // <img src="https://app.example/.env"> on another site.
        (
            "192.0.2.41:5000",
            &[
                ("sec-fetch-site", "cross-site"),
                ("sec-fetch-mode", "no-cors"),
                ("sec-fetch-dest", "image"),
            ],
        ),
        // ![x](/.env) in user content on the protected site itself.
        (
            "192.0.2.42:5000",
            &[
                ("sec-fetch-site", "same-origin"),
                ("sec-fetch-mode", "no-cors"),
                ("sec-fetch-dest", "image"),
            ],
        ),
        // A cross-site page navigating the visitor to the trap.
        (
            "192.0.2.43:5000",
            &[
                ("sec-fetch-site", "cross-site"),
                ("sec-fetch-mode", "navigate"),
                ("sec-fetch-dest", "document"),
            ],
        ),
        // Browsers without fetch metadata still send an initiating page.
        ("192.0.2.44:5000", &[("referer", "https://lure.example/")]),
        ("192.0.2.45:5000", &[("origin", "https://lure.example")]),
    ];
    for (peer, headers) in lures {
        let response = app
            .clone()
            .oneshot(trap_request(peer, headers))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    for ip in [
        "192.0.2.41",
        "192.0.2.42",
        "192.0.2.43",
        "192.0.2.44",
        "192.0.2.45",
    ] {
        assert!(!state.is_banned(ip), "page-initiated load banned {ip}");
    }

    // Direct requests (scanners, or a URL typed by the user) still ban.
    let direct: [(&str, &[(&str, &str)]); 2] = [
        ("192.0.2.46:5000", &[]),
        (
            "192.0.2.47:5000",
            &[
                ("sec-fetch-site", "none"),
                ("sec-fetch-mode", "navigate"),
                ("sec-fetch-dest", "document"),
            ],
        ),
    ];
    for (peer, headers) in direct {
        let response = app
            .clone()
            .oneshot(trap_request(peer, headers))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    assert!(state.is_banned("192.0.2.46"));
    assert!(state.is_banned("192.0.2.47"));
}
