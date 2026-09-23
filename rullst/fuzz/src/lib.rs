//! Executable fuzz contracts, also exercised by deterministic facade tests.
//! No provider accounts, sockets, environment mutation or application files.

use axum::{body::Body, http::Request};
use rullst::{BroadcastManager, TenantConfig, TenantLayer, TenantRealtime, TenantStrategy};
use std::{
    convert::Infallible,
    sync::{Arc, OnceLock},
};
use tower::{Layer, ServiceExt};

const TEST_KEY: &[u8] = b"0123456789abcdefghijklmnopqrstuv";

pub fn session(data: &[u8]) -> i32 {
    assert!(rullst::auth::validate_app_key(TEST_KEY).is_ok());
    let data = &data[..data.len().min(2048)];
    let _ = rullst::auth::validate_app_key(data);
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = rullst::auth::decrypt_session(text, TEST_KEY);
        // Reach the decoder even when mutations have not discovered the prefix.
        let _ = rullst::auth::decrypt_session(&format!("v1.{text}"), TEST_KEY);
    }
    let token = rullst::auth::encrypt_session(42, TEST_KEY).expect("valid fixture key");
    let user = rullst::auth::decrypt_session(&token, TEST_KEY).expect("valid session round trip");
    let mut forged = token.into_bytes();
    // The first encoded nonce character carries real bits, not padding bits.
    forged[3] = if forged[3] == b'A' { b'B' } else { b'A' };
    let forged = String::from_utf8(forged).expect("token remains ASCII");
    assert!(rullst::auth::decrypt_session(&forged, TEST_KEY).is_err());
    user
}

pub fn config(text: &str) -> (Option<Vec<u8>>, bool) {
    let key = rullst::auth::parse_app_key_from_toml(text);
    if let Some(bytes) = &key {
        let _ = rullst::auth::validate_app_key(bytes);
    }
    let parsed = rullst::config::RullstConfig::from_toml(text);
    if let Ok(value) = &parsed {
        let _ = value.validate();
    }
    let _ = rullst::config::Environment::resolve(Some(text), None, None);
    (key, parsed.is_ok())
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("local test runtime")
    })
}

pub fn tenants(hint: &str) -> [bool; 3] {
    let mut granted = [false; 3];
    for trusted in [false, true] {
        for (index, strategy) in [
            TenantStrategy::Header,
            TenantStrategy::Subdomain,
            TenantStrategy::Parameter,
        ]
        .into_iter()
        .enumerate()
        {
            let mut request = Request::builder();
            match strategy {
                TenantStrategy::Header => {
                    request = request.header("X-Tenant-ID", hint);
                }
                TenantStrategy::Subdomain => {
                    request = request.header("Host", format!("{hint}.example.com"));
                }
                TenantStrategy::Parameter => {
                    request = request.uri(format!("/?tenant_id={}", urlencoding::encode(hint)));
                }
                _ => unreachable!("fixture lists only the three declared strategies"),
            }
            let Ok(mut request) = request.body(Body::empty()) else {
                continue;
            };
            if trusted {
                request.extensions_mut().insert(
                    rullst::security::TenantMembership::try_new(["tenant-a"])
                        .expect("authenticated fixture membership"),
                );
            }
            let inner = tower::service_fn(|_: Request<Body>| async {
                assert_eq!(
                    rullst::multitenant::current_tenant_id().as_deref(),
                    Some("tenant-a")
                );
                Ok::<_, Infallible>(axum::http::Response::new(Body::empty()))
            });
            let service = TenantLayer::new(TenantConfig::new(strategy)).layer(inner);
            let response = runtime()
                .block_on(service.oneshot(request))
                .expect("infallible test service");
            if trusted {
                granted[index] = response.status().is_success();
            } else {
                assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
            }
        }
    }
    assert_eq!(rullst::multitenant::current_tenant_id(), None);
    granted
}

pub fn realtime(payload: &str) -> Result<String, rullst::RealtimeError> {
    let manager = Arc::new(BroadcastManager::new());
    let a = TenantRealtime::from_context(
        Arc::clone(&manager),
        &rullst::security::TenantContext::try_new("tenant-a").expect("fixture tenant"),
    );
    let b = TenantRealtime::from_context(
        manager,
        &rullst::security::TenantContext::try_new("tenant-b").expect("fixture tenant"),
    );
    let mut own = a.subscribe("room").expect("valid fixture room");
    let mut other = b.subscribe("room").expect("valid fixture room");
    let _ = a.namespaced_channel(payload);
    let result = a.publish("room", "message", payload);
    assert!(matches!(
        other.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    result?;
    let received = own
        .try_recv()
        .expect("accepted publication must be delivered");
    assert_eq!(
        received.channel,
        a.namespaced_channel("room").expect("fixture room")
    );
    assert_eq!(received.event, "message");
    Ok(received.payload)
}
