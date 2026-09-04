#![cfg(all(
    feature = "auth-sqlite",
    feature = "capital-quota-sql",
    feature = "mail-sqlite",
    feature = "messaging-sqlite",
    feature = "oauth-sqlite",
    feature = "queue-sqlite"
))]
#![allow(clippy::expect_used)]

use rullst::ApplicationLifecycle;
use rullst::auth::SqliteJwtRevocationStore;
use rullst::capital::{BillingSubject, QuotaRequest, QuotaStore as _, SqlQuotaStore};
use rullst::connect::prelude::SecretString;
use rullst::connect::{
    RefreshableTokenState, SqliteTokenSnapshotStore, TokenSnapshotBinding, TokenSnapshotKey,
};
use rullst::mail::{
    MutableSuppressionStore as _, SqliteSuppressionStore, SuppressionEvent, SuppressionReason,
    SuppressionStore as _,
};
use rullst::messaging::{
    BrokerConfig, MessageBroker as _, MessagingKeyring, MessagingStorageKey, PublishRequest,
    ReceiveRequest, SqliteBroker, StartPosition, SubscriptionRequest,
};
use rullst::orm::sqlx::{Executor as _, SqlitePool};
use rullst::queue::{QueueDriver as _, SqliteDriver};
use rullst::security::TenantContext;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const COMPONENTS: [&str; 6] = ["auth", "capital", "connect", "mail", "messaging", "queue"];
const ACCESS_TOKEN: &str = "facade-access-secret-3a1f";
const REFRESH_TOKEN: &str = "facade-refresh-secret-9c7e";
const MESSAGE_PAYLOAD: &[u8] = b"facade-message-secret-5d2b";

fn temporary_database() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock follows the Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-facade-recovery-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn database_url(path: &Path) -> String {
    let portable_path = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    let url = format!("sqlite:///{portable_path}");
    #[cfg(not(windows))]
    let url = format!("sqlite://{portable_path}");
    url
}

fn remove_database(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = PathBuf::from(format!("{}{suffix}", path.display()));
        match std::fs::remove_file(candidate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("remove SQLite fixture: {error}"),
        }
    }
}

fn assert_database_excludes(path: &Path, forbidden: &[u8]) {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = PathBuf::from(format!("{}{suffix}", path.display()));
        let bytes = match std::fs::read(candidate) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => panic!("read SQLite fixture: {error}"),
        };
        assert!(
            !bytes
                .windows(forbidden.len())
                .any(|window| window == forbidden),
            "forbidden plaintext was retained by the shared local profile"
        );
    }
}

fn token_material() -> (
    TokenSnapshotBinding,
    TokenSnapshotKey,
    RefreshableTokenState,
) {
    let binding =
        TokenSnapshotBinding::try_new("github", "facade-user").expect("valid token binding");
    let key = TokenSnapshotKey::try_new("facade-oauth-2026-a", [41; 32])
        .expect("valid token encryption key");
    let state = RefreshableTokenState::try_new(
        "provider-user-7",
        SecretString::from(ACCESS_TOKEN.to_string()),
        SecretString::from(REFRESH_TOKEN.to_string()),
        1_800_000_000,
        3_600,
    )
    .expect("valid initial token state");
    (binding, key, state)
}

fn messaging_config() -> BrokerConfig {
    BrokerConfig::try_new("facade-recovery")
        .and_then(|config| config.with_limits(32, 8, 3, 1_024))
        .expect("bounded broker configuration")
}

fn messaging_keyring() -> MessagingKeyring {
    MessagingKeyring::new(
        MessagingStorageKey::try_new("facade-messaging-2026-a", [73; 32])
            .expect("valid messaging encryption key"),
    )
}

fn publication() -> PublishRequest {
    PublishRequest::try_new(
        "audit",
        "facade.recovery",
        "facade/recovery/1",
        MESSAGE_PAYLOAD.to_vec(),
    )
    .expect("bounded publication")
}

fn quota_request() -> QuotaRequest {
    let tenant = TenantContext::try_new("facade-tenant").expect("valid tenant context");
    let subject = BillingSubject::from_tenant(&tenant).expect("valid billing subject");
    QuotaRequest::try_new(subject, "courses", "course-create-1", 1, 3).expect("valid quota request")
}

fn mark_component(lifecycle: &ApplicationLifecycle, component: &str) {
    lifecycle
        .set_component_ready(component, true)
        .expect("registered component becomes ready");
}

#[tokio::test]
async fn facade_shared_local_profile_recovers_and_fails_closed() {
    let path = temporary_database();
    let url = database_url(&path);
    let lifecycle =
        ApplicationLifecycle::with_required_components(COMPONENTS).expect("bounded lifecycle");
    lifecycle.mark_ready().expect("startup phase completes");
    assert!(!lifecycle.snapshot().ready);

    let auth = SqliteJwtRevocationStore::connect(&url, 32)
        .await
        .expect("open auth store");
    auth.revoke_subject_before("facade-user", 4)
        .await
        .expect("persist subject revocation");
    mark_component(&lifecycle, "auth");

    let quota = SqlQuotaStore::connect(&url)
        .await
        .expect("open quota store");
    quota.prepare_schema().await.expect("prepare quota schema");
    let quota_request = quota_request();
    let first_grant = quota.reserve(&quota_request).await.expect("reserve quota");
    assert!(!first_grant.is_replay());
    mark_component(&lifecycle, "capital");

    let (binding, token_key, token_state) = token_material();
    let connect = SqliteTokenSnapshotStore::connect(&url, 32)
        .await
        .expect("open token store");
    connect
        .insert_initial(&binding, &token_state, &token_key)
        .await
        .expect("persist encrypted token state");
    mark_component(&lifecycle, "connect");

    let mail = SqliteSuppressionStore::connect(&url, 32, 64)
        .await
        .expect("open mail suppression store");
    let observed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock follows the Unix epoch")
        .as_secs();
    mail.record(
        SuppressionEvent::try_new(
            "postmark",
            "facade-bounce-1",
            "blocked@example.test",
            SuppressionReason::HardBounce,
            observed_at,
        )
        .expect("valid suppression event"),
    )
    .await
    .expect("persist suppression");
    mark_component(&lifecycle, "mail");

    let broker = SqliteBroker::connect_encrypted(&url, messaging_config(), messaging_keyring())
        .await
        .expect("open encrypted broker");
    broker
        .subscribe(
            SubscriptionRequest::try_new("audit", "workers", StartPosition::Earliest)
                .expect("valid subscription"),
        )
        .await
        .expect("persist subscription");
    let first_receipt = broker
        .publish(publication())
        .await
        .expect("persist encrypted message");
    mark_component(&lifecycle, "messaging");

    let queue = SqliteDriver::new(&url).await.expect("open queue");
    queue
        .push("facade-job-1", "rebuild_search", r#"{"course":7}"#)
        .await
        .expect("persist queued job");
    mark_component(&lifecycle, "queue");
    assert!(lifecycle.snapshot().ready);

    auth.close().await;
    quota.pool().close().await;
    drop(quota);
    connect.close().await;
    mail.close().await;
    broker.close().await;
    queue.get_pool().close().await;
    drop(broker);
    drop(queue);

    assert_database_excludes(&path, ACCESS_TOKEN.as_bytes());
    assert_database_excludes(&path, REFRESH_TOKEN.as_bytes());
    assert_database_excludes(&path, MESSAGE_PAYLOAD);

    let recovered_lifecycle =
        ApplicationLifecycle::with_required_components(COMPONENTS).expect("recovery lifecycle");
    recovered_lifecycle
        .mark_ready()
        .expect("recovery startup completes");

    let auth = SqliteJwtRevocationStore::connect(&url, 32)
        .await
        .expect("reopen auth store");
    assert_eq!(
        auth.snapshot()
            .await
            .expect("read recovered revocations")
            .subject_revocations(),
        1
    );
    mark_component(&recovered_lifecycle, "auth");

    let quota = SqlQuotaStore::connect(&url)
        .await
        .expect("reopen quota store");
    quota.prepare_schema().await.expect("verify quota schema");
    let replay = quota
        .reserve(&quota_request)
        .await
        .expect("recover idempotent quota claim");
    assert!(replay.is_replay());
    assert_eq!(replay.used_after(), first_grant.used_after());
    mark_component(&recovered_lifecycle, "capital");

    let connect = SqliteTokenSnapshotStore::connect(&url, 32)
        .await
        .expect("reopen token store");
    let recovered_token = connect
        .load(&binding, &token_key)
        .await
        .expect("authenticate recovered token state")
        .expect("token state exists");
    assert_eq!(recovered_token.generation(), 0);
    assert_eq!(recovered_token.provider_user_id(), "provider-user-7");
    mark_component(&recovered_lifecycle, "connect");

    let mail = SqliteSuppressionStore::connect(&url, 32, 64)
        .await
        .expect("reopen mail store");
    let suppression = mail
        .lookup("blocked@example.test")
        .await
        .expect("read recovered suppression")
        .expect("suppression exists");
    assert_eq!(suppression.reason(), SuppressionReason::HardBounce);
    mark_component(&recovered_lifecycle, "mail");

    let broker = SqliteBroker::connect_encrypted(&url, messaging_config(), messaging_keyring())
        .await
        .expect("reopen encrypted broker");
    let duplicate = broker
        .publish(publication())
        .await
        .expect("recover idempotent publication");
    assert!(duplicate.is_duplicate());
    assert_eq!(duplicate.id(), first_receipt.id());
    let deliveries = broker
        .receive(
            ReceiveRequest::try_new("audit", "workers", "worker-a", 1, Duration::from_secs(30))
                .expect("valid receive request"),
        )
        .await
        .expect("receive recovered message");
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].envelope().payload(), MESSAGE_PAYLOAD);
    broker
        .ack(deliveries[0].ack_token())
        .await
        .expect("ack recovered message");
    mark_component(&recovered_lifecycle, "messaging");

    let queue = SqliteDriver::new(&url).await.expect("reopen queue");
    assert_eq!(queue.pending_count().await.expect("pending jobs"), 1);
    let job = queue
        .pop()
        .await
        .expect("recover queued job")
        .expect("queued job exists");
    assert_eq!(job.id, "facade-job-1");
    queue
        .mark_complete(&job.id)
        .await
        .expect("complete recovered job");
    mark_component(&recovered_lifecycle, "queue");
    assert!(recovered_lifecycle.snapshot().ready);

    let repair_pool = SqlitePool::connect(&url)
        .await
        .expect("open corruption fixture connection");
    repair_pool
        .execute("UPDATE rullst_connect_token_snapshots SET envelope = 'corrupt-envelope'")
        .await
        .expect("inject isolated token corruption");
    repair_pool.close().await;
    let error = connect
        .load(&binding, &token_key)
        .await
        .expect_err("corrupt encrypted token state fails closed");
    let diagnostic = error.to_string();
    assert!(!diagnostic.contains(ACCESS_TOKEN));
    assert!(!diagnostic.contains(REFRESH_TOKEN));
    assert!(!diagnostic.contains(&url));
    assert!(
        mail.lookup("blocked@example.test")
            .await
            .expect("unrelated mail state remains readable")
            .is_some()
    );

    auth.close().await;
    quota.pool().close().await;
    drop(quota);
    connect.close().await;
    mail.close().await;
    broker.close().await;
    queue.get_pool().close().await;
    drop(broker);
    drop(queue);
    remove_database(&path);
}
