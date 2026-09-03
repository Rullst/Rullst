#![cfg(feature = "sqlite")]

use async_trait::async_trait;
use rullst_connect::prelude::*;
use rullst_connect::provider::ExchangeParams;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct RefreshProvider {
    refreshed_user: ConnectUser,
}

#[async_trait]
impl Provider for RefreshProvider {
    fn redirect_url(&self) -> String {
        "https://identity.example.test/authorize".to_string()
    }

    async fn get_user(&self, _params: ExchangeParams<'_>) -> Result<ConnectUser, ConnectError> {
        Ok(self.refreshed_user.clone())
    }

    async fn get_user_from_token(&self, _access_token: &str) -> Result<ConnectUser, ConnectError> {
        Ok(self.refreshed_user.clone())
    }

    fn token_url(&self) -> String {
        "https://identity.example.test/token".to_string()
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<ConnectUser, ConnectError> {
        Ok(self.refreshed_user.clone())
    }
}

fn user(access_token: &str, refresh_token: &str) -> ConnectUser {
    ConnectUser {
        id: "provider-user".to_string(),
        name: "Stored user".to_string(),
        email: None,
        email_verified: None,
        avatar_url: None,
        raw_data: json!({}),
        access_token: SecretString::from(access_token.to_string()),
        refresh_token: Some(SecretString::from(refresh_token.to_string())),
        expires_in: Some(10),
    }
}

fn temporary_database() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rullst-connect-public-token-store-{}-{nonce}.sqlite",
        std::process::id()
    ))
}

fn database_url(path: &Path) -> String {
    format!("sqlite://{}", path.to_string_lossy().replace('\\', "/"))
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

#[tokio::test]
async fn public_store_recovers_and_cas_persists_a_provider_refresh() {
    let path = temporary_database();
    let url = database_url(&path);
    let first = SqliteTokenSnapshotStore::connect(&url, 4)
        .await
        .expect("open first store");
    let second = SqliteTokenSnapshotStore::connect(&url, 4)
        .await
        .expect("open second store");
    let binding =
        TokenSnapshotBinding::try_new("github", "application-user").expect("valid binding");
    let key = TokenSnapshotKey::try_new("oauth-2026-a", [47; 32]).expect("valid key");
    let initial = RefreshableTokenState::from_user_at(&user("access-0", "refresh-0"), 100)
        .expect("valid initial state");
    first
        .insert_initial(&binding, &initial, &key)
        .await
        .expect("insert generation zero");

    let recovered = second
        .load(&binding, &key)
        .await
        .expect("load generation")
        .expect("generation exists");
    let provider = RefreshProvider {
        refreshed_user: user("access-1", "refresh-1"),
    };
    let session = AutoRefreshingSession::new(&provider, recovered);
    let lease = session
        .access_token_at(110)
        .await
        .expect("refresh expired token");
    assert!(lease.was_refreshed());
    assert_eq!(lease.generation(), 1);
    assert_eq!(lease.access_token().expose_secret(), "access-1");
    let replacement = session.state_snapshot().await;
    second
        .compare_and_swap(&binding, 0, &replacement, &key)
        .await
        .expect("persist exact successor");
    first.close().await;
    second.close().await;

    let reopened = SqliteTokenSnapshotStore::connect(&url, 4)
        .await
        .expect("reopen store");
    let final_state = reopened
        .load(&binding, &key)
        .await
        .expect("load final generation")
        .expect("final generation exists");
    assert_eq!(final_state.generation(), 1);
    assert_eq!(final_state.access_token().expose_secret(), "access-1");
    assert_eq!(final_state.refresh_token().expose_secret(), "refresh-1");
    reopened.close().await;
    remove_database(&path);
}
