//! Service-level API token boundaries over a disposable SQLite file.
use super::*;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const T: u64 = 1_800_000_000;

/// Returns queued readings first, then repeats the last one.
struct Script {
    queued: Mutex<VecDeque<u64>>,
    last: AtomicU64,
}
impl Script {
    fn at(time: u64) -> Self {
        Self {
            queued: Mutex::new(VecDeque::new()),
            last: AtomicU64::new(time),
        }
    }
    fn set(&self, time: u64) {
        self.queued.lock().unwrap().clear();
        self.last.store(time, Ordering::SeqCst);
    }
    fn queue(&self, readings: &[u64]) {
        self.queued.lock().unwrap().extend(readings);
    }
}
impl AuthClock for Script {
    fn now(&self) -> Result<u64, RecoveryError> {
        if let Some(reading) = self.queued.lock().unwrap().pop_front() {
            self.last.store(reading, Ordering::SeqCst);
        }
        Ok(self.last.load(Ordering::SeqCst))
    }
}

fn scopes() -> ApiScopes {
    ApiScopes::new(["orders:read", "orders:write"]).unwrap()
}
fn read() -> ApiScopes {
    ApiScopes::new(["orders:read"]).unwrap()
}
fn label() -> SessionLabel {
    SessionLabel::new("Reporting integration").unwrap()
}
fn config() -> ApiTokenConfig {
    ApiTokenConfig::new("api-unit", scopes(), 100, 3600).unwrap()
}
fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}

/// A fresh database file; keep the directory alive for the test.
async fn service() -> (tempfile::TempDir, String, ApiTokenService) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tokens.db");
    let encoded: String =
        url::form_urlencoded::byte_serialize(path.to_str().unwrap().as_bytes()).collect();
    let url = format!("sqlite:{}", encoded.replace('+', "%20"));
    let service = ApiTokenService::initialize(&url, keys(), config())
        .await
        .unwrap();
    (directory, url, service)
}

async fn account(service: &ApiTokenService, now: u64) -> AuthenticatedRecoveryAccount {
    let password = "Fixture_0123456789abcdef0123456789";
    service
        .accounts()
        .register_account("owner-1", "owner@example.com", password, now)
        .await
        .unwrap();
    service
        .accounts()
        .authenticate("owner@example.com", password)
        .await
        .unwrap()
        .unwrap()
}

async fn stored_rows(service: &ApiTokenService) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM rullst_api_tokens")
        .fetch_one(&service.store.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn lifetimes_revisions_and_inventory_accept_their_limits() {
    let (_directory, _, service) = service().await;
    let clock = Script::at(T);
    let owner = account(&service, T).await;
    let longest = service
        .issue(&owner, read(), label(), 3_600, &clock)
        .await
        .expect("the configured maximum lifetime is allowed");
    assert_eq!(longest.metadata().expires_at(), T + 3_600);
    assert_eq!(
        service
            .rotate(&owner, longest.metadata().id(), 0, 600, &clock)
            .await
            .err(),
        Some(RecoveryError::InvalidInput)
    );
    for _ in 1..20 {
        service
            .issue(&owner, read(), label(), 600, &clock)
            .await
            .unwrap();
    }
    clock.set(T + 1);
    let listed = service.inventory(&owner, &clock).await.unwrap();
    assert_eq!(listed.len(), 20, "an account's twenty tokens are listed");
    assert!(listed.iter().all(|token| token.issued_at() == T));
}

#[tokio::test]
async fn expiry_is_rechecked_before_and_after_commit() {
    let (_directory, _, service) = service().await;
    let clock = Script::at(T);
    let owner = account(&service, T).await;
    // The clock reaches the new token's expiry before the insert commits.
    clock.queue(&[T, T, T + 1]);
    assert_eq!(
        service
            .issue(&owner, read(), label(), 1, &clock)
            .await
            .err(),
        Some(RecoveryError::InvalidAction)
    );
    assert_eq!(stored_rows(&service).await, 0, "the insert was rolled back");

    clock.set(T);
    let token = service
        .issue(&owner, read(), label(), 100, &clock)
        .await
        .unwrap();
    // Inventory hides a token that expired while its transaction committed.
    clock.queue(&[T + 50, T + 50, T + 50, T + 100]);
    assert!(service.inventory(&owner, &clock).await.unwrap().is_empty());
    clock.set(T + 50);
    clock.queue(&[T + 50, T + 50, T + 50, T + 99]);
    assert_eq!(service.inventory(&owner, &clock).await.unwrap().len(), 1);
    // Verification fails when expiry arrives after its commit.
    clock.set(T + 99);
    clock.queue(&[T + 99, T + 99, T + 99, T + 100]);
    assert_eq!(
        service
            .verify(token.expose_bearer(), &read(), &clock)
            .await
            .err(),
        Some(RecoveryError::InvalidAction)
    );
    clock.set(T + 99);
    assert!(
        service
            .verify(token.expose_bearer(), &read(), &clock)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn purge_removes_expired_rows_only() {
    let (_directory, _, service) = service().await;
    let clock = Script::at(T);
    let owner = account(&service, T).await;
    for lifetime in [60, 600] {
        service
            .issue(&owner, read(), label(), lifetime, &clock)
            .await
            .unwrap();
    }
    clock.set(T + 61);
    service.purge_expired(&clock).await.unwrap();
    assert_eq!(stored_rows(&service).await, 1);
}

#[tokio::test]
async fn durability_settings_and_stored_capacity_are_rechecked() {
    let (_directory, url, service) = service().await;
    let clock = Script::at(T);
    for (weaken, restore) in [
        ("PRAGMA synchronous = NORMAL", "PRAGMA synchronous = FULL"),
        ("PRAGMA journal_mode = DELETE", "PRAGMA journal_mode = WAL"),
    ] {
        sqlx::query(weaken)
            .execute(&service.store.pool)
            .await
            .unwrap();
        assert_eq!(
            service.purge_expired(&clock).await.err(),
            Some(RecoveryError::Configuration),
            "{weaken}"
        );
        sqlx::query(restore)
            .execute(&service.store.pool)
            .await
            .unwrap();
        service.purge_expired(&clock).await.unwrap();
    }
    // A stored capacity edited outside the binding no longer matches.
    sqlx::query("UPDATE rullst_api_token_control SET capacity = 5")
        .execute(&service.store.pool)
        .await
        .unwrap();
    assert_eq!(
        service.purge_expired(&clock).await.err(),
        Some(RecoveryError::Configuration)
    );
    service.close().await;
    assert_eq!(
        ApiTokenService::connect(&url, keys(), config()).await.err(),
        Some(RecoveryError::Configuration)
    );
}

#[tokio::test]
async fn stored_rows_are_decoded_only_within_their_bounds() {
    let (_directory, _, service) = service().await;
    let clock = Script::at(T);
    let owner = account(&service, T).await;
    let id = service
        .issue(&owner, read(), label(), 60, &clock)
        .await
        .unwrap()
        .metadata()
        .id()
        .clone();
    let decode = |scopes: &'static str, revision: i64, times: [i64; 3]| {
        let id = id.as_str().to_owned();
        let service = &service;
        async move {
            let row = sqlx::query(
                "SELECT $1 AS id,$2 AS label,$3 AS scopes,$4 AS revision,$5 AS created_at,\
                 $6 AS issued_at,$7 AS expires_at",
            )
            .bind(id)
            .bind("Reporting integration")
            .bind(scopes)
            .bind(revision)
            .bind(times[0])
            .bind(times[1])
            .bind(times[2])
            .fetch_one(&service.store.pool)
            .await
            .unwrap();
            service.row(&row).map(|metadata| metadata.expires_at())
        }
    };
    assert_eq!(decode("orders:read", 1, [0, 0, 10]).await, Ok(10));
    assert_eq!(decode("orders:read", 1, [100, 100, 3_700]).await, Ok(3_700));
    for (scopes, revision, times) in [
        ("orders:read", 0, [100, 100, 200]),
        ("orders:read", 1, [-1, 0, 10]),
        ("orders:read", 1, [100, 50, 60]),
        ("orders:read", 1, [100, 100, 100]),
        ("orders:read", 1, [1_000, 1_000, 4_601]),
        ("admin:all", 1, [100, 100, 200]),
    ] {
        assert_eq!(
            decode(scopes, revision, times).await,
            Err(RecoveryError::Configuration),
            "{scopes} {revision} {times:?}"
        );
    }
}

#[test]
fn the_longest_scope_list_round_trips() {
    let scopes =
        ApiScopes::new((0..32).map(|index| format!("s{index:02}{}", "x".repeat(61)))).unwrap();
    let encoded = scopes.encode();
    assert_eq!(encoded.len(), 2_079);
    assert_eq!(ApiScopes::decode(&encoded), Ok(scopes));
    assert_eq!(
        ApiScopes::decode(&format!("{encoded}x")),
        Err(RecoveryError::Configuration)
    );
}

#[tokio::test]
async fn debug_output_names_only_public_bounds() {
    let (_directory, _, service) = service().await;
    let clock = Script::at(T);
    let owner = account(&service, T).await;
    assert_eq!(
        format!("{:?}", config()),
        "ApiTokenConfig { capacity: 100, maximum_lifetime_seconds: 3600, .. }"
    );
    let token = service
        .issue(&owner, read(), label(), 60, &clock)
        .await
        .unwrap();
    let principal = service
        .verify(token.expose_bearer(), &read(), &clock)
        .await
        .unwrap();
    assert_eq!(format!("{token:?}"), "IssuedApiToken([REDACTED])");
    assert_eq!(
        format!("{:?}", token.metadata()),
        "ApiTokenMetadata([REDACTED])"
    );
    assert_eq!(
        format!("{:?}", token.metadata().id()),
        "ApiTokenId([REDACTED])"
    );
    assert_eq!(format!("{principal:?}"), "ApiTokenPrincipal([REDACTED])");
}

#[tokio::test]
async fn machine_verifier_rejects_each_ambient_credential_on_its_own() {
    use axum::{body::Body, extract::Request, http::header};
    use rullst_core::security::{MachineEndpointError, MachineRequestVerifier};

    let (_directory, _, service) = service().await;
    let system = super::super::SystemAuthClock;
    let owner = account(&service, system.now().unwrap()).await;
    let token = service
        .issue(&owner, read(), label(), 600, &system)
        .await
        .unwrap();
    let verifier = service.machine_verifier(read()).unwrap();
    let request = |extra: Option<(&str, &str)>| {
        let mut builder = Request::builder().method("POST").uri("/api/orders").header(
            header::AUTHORIZATION,
            format!("Bearer {}", token.expose_bearer()),
        );
        if let Some((name, value)) = extra {
            builder = builder.header(name, value);
        }
        builder.body(Body::empty()).unwrap()
    };
    for extra in [
        ("cookie", "session=ambient"),
        ("origin", "https://evil.example"),
        ("sec-fetch-site", "same-origin"),
    ] {
        assert!(
            matches!(
                verifier.verify(request(Some(extra))).await,
                Err(MachineEndpointError::Unauthorized)
            ),
            "{}",
            extra.0
        );
    }
    let verified = verifier.verify(request(None)).await.unwrap();
    assert_eq!(
        verified
            .extensions()
            .get::<ApiTokenPrincipal>()
            .unwrap()
            .subject(),
        "owner-1"
    );
}
