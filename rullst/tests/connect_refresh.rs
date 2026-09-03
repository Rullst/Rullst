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

#[cfg(feature = "oauth-sqlite")]
#[tokio::test]
async fn umbrella_exposes_the_opt_in_sqlite_token_store() {
    use rullst::connect::SqliteTokenSnapshotStore;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "rullst-facade-connect-store-{}-{nonce}.sqlite",
        std::process::id()
    ));
    let url = format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"));
    let store = SqliteTokenSnapshotStore::connect(url, 2)
        .await
        .expect("open facade token store");
    let snapshot = store.snapshot().await.expect("read store metadata");
    assert_eq!(snapshot.entries(), 0);
    assert_eq!(snapshot.max_entries(), 2);
    store.close().await;
    std::fs::remove_file(path).expect("remove SQLite fixture");
}
