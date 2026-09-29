#![cfg(any(feature = "recovery-sqlite", feature = "recovery-postgres"))]

#[cfg(feature = "recovery-sqlite")]
use rullst_auth::recovery::RecoveryDeliveryFailure;

use rullst_auth::recovery::{RecoveryError, RecoveryNoticeKind, RecoverySecrets, SqlRecoveryStore};

const EMAIL: &str = "recovery@example.com";
fn fixture_password() -> String {
    format!("Fixture_{:032x}", rand::random::<u128>())
}

fn keys() -> RecoverySecrets {
    RecoverySecrets::new([3; 32], [9; 32]).unwrap()
}

#[cfg(feature = "recovery-sqlite")]
fn database_url(path: &std::path::Path) -> String {
    // AnyPool parses a URL before SQLite sees the filename. Keep drive letters
    // out of the authority and encode reserved path bytes (including '%'/'#').
    let encoded: String =
        url::form_urlencoded::byte_serialize(path.to_str().expect("UTF-8 fixture path").as_bytes())
            .collect();
    format!("sqlite:{}?mode=rwc", encoded.replace('+', "%20"))
}

#[cfg(feature = "recovery-sqlite")]
#[test]
fn sqlite_fixture_urls_preserve_windows_and_unix_filenames() {
    for filename in [
        r"C:\Users\Example User\recovery 100%#1.db",
        "/tmp/recovery 100%#1.db",
    ] {
        let path = std::path::Path::new(filename);
        let options: sqlx::any::AnyConnectOptions = database_url(path).parse().unwrap();
        assert!(options.database_url.host_str().is_none());
        assert!(options.database_url.fragment().is_none());
        let sqlite: sqlx::sqlite::SqliteConnectOptions =
            options.database_url.as_str().parse().unwrap();
        assert_eq!(sqlite.get_filename(), path);
    }
}

async fn consume_welcome(store: &SqlRecoveryStore, now: u64) {
    let welcome = store.claim_notice(now).await.unwrap().unwrap();
    assert_eq!(welcome.notice().kind(), RecoveryNoticeKind::Welcome);
    store.complete_notice(&welcome, now + 1).await.unwrap();
}

async fn recovery_contract(url: &str) {
    let old = fixture_password();
    let new = fixture_password();
    let now = 1_800_000_000;
    let store = SqlRecoveryStore::connect(url, keys()).await.unwrap();
    store.migrate().await.unwrap();
    store
        .register_account("member-one", EMAIL, old.as_str(), now)
        .await
        .unwrap();
    consume_welcome(&store, now).await;
    let account = store
        .authenticate(EMAIL, old.as_str())
        .await
        .unwrap()
        .unwrap();
    let session = store.create_session(&account, now, 86400).await.unwrap();
    assert_eq!(
        store
            .verify_session(session.expose(), now + 1)
            .await
            .unwrap()
            .as_deref(),
        Some("member-one")
    );

    let start = std::time::Instant::now();
    let absent = store
        .request_password_reset("unknown@example.com", now + 2)
        .await
        .unwrap();
    assert!(start.elapsed() >= std::time::Duration::from_millis(240));
    assert_eq!(store.outbox_snapshot().await.unwrap().pending, 0);
    assert_eq!(
        store.request_password_reset(EMAIL, now + 3).await.unwrap(),
        absent
    );
    let first = store.claim_notice(now + 4).await.unwrap().unwrap();
    let obsolete = first.notice().token().unwrap().expose().to_owned();
    assert_eq!(obsolete.len(), 43);
    assert!(!format!("{first:?}").contains(&obsolete));
    assert!(!format!("{first:?}").contains(EMAIL));
    store.complete_notice(&first, now + 5).await.unwrap();

    store.request_password_reset(EMAIL, now + 6).await.unwrap();
    assert_eq!(
        store
            .complete_password_reset(&obsolete, new.as_str(), now + 7)
            .await,
        Err(RecoveryError::InvalidAction)
    );
    let second = store.claim_notice(now + 7).await.unwrap().unwrap();
    let token = second.notice().token().unwrap().expose().to_owned();
    assert_ne!(token, obsolete);
    store.complete_notice(&second, now + 8).await.unwrap();
    assert!(
        store
            .complete_password_reset(session.expose(), new.as_str(), now + 9)
            .await
            .is_err()
    );

    let first_worker = store.clone();
    let second_worker = store.clone();
    let (one, two) = tokio::join!(
        first_worker.complete_password_reset(&token, new.as_str(), now + 10),
        second_worker.complete_password_reset(&token, new.as_str(), now + 10),
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    assert_eq!(
        store
            .verify_session(session.expose(), now + 11)
            .await
            .unwrap(),
        None
    );
    assert!(
        store
            .create_session(&account, now + 11, 3600)
            .await
            .is_err()
    );
    assert!(
        store
            .authenticate(EMAIL, old.as_str())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .authenticate(EMAIL, new.as_str())
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        store
            .complete_password_reset(&token, new.as_str(), now + 11)
            .await,
        Err(RecoveryError::InvalidAction)
    );
    let changed = store.claim_notice(now + 12).await.unwrap().unwrap();
    assert_eq!(changed.notice().kind(), RecoveryNoticeKind::PasswordChanged);
    store.complete_notice(&changed, now + 13).await.unwrap();

    // The third permitted request expires; the fourth shares the public response
    // but cannot replace it or produce more deliveries within the same window.
    store.request_password_reset(EMAIL, now + 14).await.unwrap();
    let expiring = store.claim_notice(now + 15).await.unwrap().unwrap();
    let expired_token = expiring.notice().token().unwrap().expose().to_owned();
    store.complete_notice(&expiring, now + 16).await.unwrap();
    assert_eq!(
        store.request_password_reset(EMAIL, now + 17).await.unwrap(),
        absent
    );
    assert!(store.claim_notice(now + 18).await.unwrap().is_none());
    assert_eq!(
        store
            .complete_password_reset(&expired_token, new.as_str(), now + 1214)
            .await,
        Err(RecoveryError::InvalidAction)
    );
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn sqlite_recovery_is_atomic_single_use_and_revokes_sessions() {
    recovery_contract("sqlite::memory:").await;
}

#[cfg(feature = "recovery-postgres")]
#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database; run explicitly in the database CI contract"]
async fn postgres_recovery_contract() {
    // A dedicated disposable database; the CI database contract supplies it.
    let url = std::env::var("RULLST_RECOVERY_TEST_POSTGRES_URL")
        .expect("disposable PostgreSQL test database URL");
    recovery_contract(&url).await;
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn outbox_survives_restart_encrypts_payloads_and_fences_workers() {
    let old = fixture_password();
    let path =
        std::env::temp_dir().join(format!("rullst-recovery 100%#{}.db", rand::random::<u64>()));
    let url = database_url(&path);
    let store = SqlRecoveryStore::connect(&url, keys()).await.unwrap();
    store.migrate().await.unwrap();
    store
        .register_account("durable", EMAIL, old.as_str(), 1000)
        .await
        .unwrap();
    consume_welcome(&store, 1000).await;
    store.request_password_reset(EMAIL, 1010).await.unwrap();
    let first = store.claim_notice(1011).await.unwrap().unwrap();
    let token = first.notice().token().unwrap().expose().to_owned();
    assert!(store.claim_notice(1012).await.unwrap().is_none());
    let raw = std::fs::read(&path).unwrap();
    for secret in [EMAIL, old.as_str(), token.as_str()] {
        assert!(
            !raw.windows(secret.len())
                .any(|window| window == secret.as_bytes())
        );
    }
    store.close().await;
    let restarted = SqlRecoveryStore::connect(&url, keys()).await.unwrap();
    restarted.migrate().await.unwrap();
    let retry = restarted.claim_notice(1071).await.unwrap().unwrap();
    assert_eq!(retry.delivery_id(), first.delivery_id());
    assert_eq!(retry.notice().token().unwrap().expose(), token);
    assert_eq!(
        restarted.complete_notice(&first, 1072).await,
        Err(RecoveryError::InvalidAction)
    );
    restarted
        .fail_notice(&retry, RecoveryDeliveryFailure::Transient, 1072)
        .await
        .unwrap();
    assert!(restarted.claim_notice(1073).await.unwrap().is_none());
    let third = restarted.claim_notice(1132).await.unwrap().unwrap();
    restarted
        .fail_notice(&third, RecoveryDeliveryFailure::PermanentBounce, 1133)
        .await
        .unwrap();
    assert_eq!(restarted.outbox_snapshot().await.unwrap().failed, 1);
    restarted.request_password_reset(EMAIL, 2000).await.unwrap();
    assert!(restarted.claim_notice(2001).await.unwrap().is_none());
    let wrong = SqlRecoveryStore::connect(&url, RecoverySecrets::new([1; 32], [2; 32]).unwrap())
        .await
        .unwrap();
    assert!(wrong.migrate().await.is_err());
    assert!(
        wrong
            .register_account("injected", "other@example.com", old.as_str(), 2001)
            .await
            .is_err()
    );
    wrong.close().await;
    restarted.close().await;
    std::fs::remove_file(path).unwrap();
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn outbox_insert_failure_rolls_back_password_token_and_session_changes() {
    let old = fixture_password();
    let new = fixture_password();
    let path = std::env::temp_dir().join(format!(
        "rullst-recovery-rollback-{}.db",
        rand::random::<u64>()
    ));
    let url = database_url(&path);
    let store = SqlRecoveryStore::connect(&url, keys()).await.unwrap();
    store.migrate().await.unwrap();
    store
        .register_account("rollback", EMAIL, old.as_str(), 1000)
        .await
        .unwrap();
    consume_welcome(&store, 1000).await;
    let account = store
        .authenticate(EMAIL, old.as_str())
        .await
        .unwrap()
        .unwrap();
    let session = store.create_session(&account, 1001, 3600).await.unwrap();
    store.request_password_reset(EMAIL, 1002).await.unwrap();
    let claim = store.claim_notice(1003).await.unwrap().unwrap();
    let token = claim.notice().token().unwrap().expose().to_owned();
    store.complete_notice(&claim, 1004).await.unwrap();
    let pool = sqlx::SqlitePool::connect(&url).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_changed BEFORE INSERT ON rullst_recovery_outbox WHEN NEW.kind = 'changed' BEGIN SELECT RAISE(ABORT, 'injected test failure'); END")
        .execute(&pool).await.unwrap();
    assert_eq!(
        store
            .complete_password_reset(&token, new.as_str(), 1005)
            .await,
        Err(RecoveryError::Storage)
    );
    assert!(
        store
            .authenticate(EMAIL, old.as_str())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .authenticate(EMAIL, new.as_str())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        store
            .verify_session(session.expose(), 1006)
            .await
            .unwrap()
            .as_deref(),
        Some("rollback")
    );
    sqlx::query("DROP TRIGGER reject_changed")
        .execute(&pool)
        .await
        .unwrap();
    store
        .complete_password_reset(&token, new.as_str(), 1007)
        .await
        .unwrap();
    assert!(
        store
            .verify_session(session.expose(), 1008)
            .await
            .unwrap()
            .is_none()
    );
    pool.close().await;
    store.close().await;
    std::fs::remove_file(path).unwrap();
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn overlong_passwords_share_one_error_for_known_and_unknown_accounts() {
    // Argon2 helpers accept at most 72 bytes. An 80-byte password must not
    // distinguish a registered email (Argon2 then `Ok(None)`) from an unknown
    // one (fast `Err(Crypto)`), and registration/reset must reject it as input.
    let now = 1_800_000_000;
    let password = fixture_password();
    let overlong = "p".repeat(80);
    let store = SqlRecoveryStore::connect("sqlite::memory:", keys())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store
        .register_account("member-long", EMAIL, password.as_str(), now)
        .await
        .unwrap();
    consume_welcome(&store, now).await;

    let known = store.authenticate(EMAIL, overlong.as_str()).await;
    let unknown = store
        .authenticate("absent@example.com", overlong.as_str())
        .await;
    assert_eq!(known.err(), Some(RecoveryError::InvalidInput));
    assert_eq!(unknown.err(), Some(RecoveryError::InvalidInput));
    // A wrong in-bound password for an unknown account keeps the uniform result.
    assert!(
        store
            .authenticate("absent@example.com", password.as_str())
            .await
            .unwrap()
            .is_none()
    );

    assert_eq!(
        store
            .register_account("member-over", "over@example.com", overlong.as_str(), now)
            .await,
        Err(RecoveryError::InvalidInput)
    );
    let boundary = "b".repeat(72);
    store
        .register_account("member-edge", "edge@example.com", boundary.as_str(), now)
        .await
        .unwrap();
    assert!(
        store
            .authenticate("edge@example.com", boundary.as_str())
            .await
            .unwrap()
            .is_some()
    );

    store.request_password_reset(EMAIL, now + 1).await.unwrap();
    let mut token = None;
    while let Some(claim) = store.claim_notice(now + 2).await.unwrap() {
        if let Some(value) = claim.notice().token() {
            token = Some(value.expose().to_owned());
        }
        store.complete_notice(&claim, now + 3).await.unwrap();
    }
    let token = token.expect("reset notice");
    assert_eq!(
        store
            .complete_password_reset(&token, overlong.as_str(), now + 4)
            .await,
        Err(RecoveryError::InvalidInput)
    );
    // The rejected reset leaves the credential usable with a bounded password.
    store
        .complete_password_reset(&token, password.as_str(), now + 5)
        .await
        .unwrap();
    store.close().await;
}

#[cfg(feature = "recovery-postgres")]
#[tokio::test]
async fn postgres_urls_use_the_hardened_connection_policy_before_connecting() {
    // The store shares the email-login/API-token PostgreSQL policy: unknown
    // query options and fragments fail as configuration without any network I/O.
    for url in [
        "postgres://db.invalid/accounts?application_name=recovery",
        "postgres://db.invalid/accounts?sslmode=verify-full&sslmode=disable",
        "postgresql://db.invalid/accounts#fragment",
    ] {
        let started = std::time::Instant::now();
        assert_eq!(
            SqlRecoveryStore::connect(url, keys()).await.err(),
            Some(RecoveryError::Configuration),
            "{url}"
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }
}

#[cfg(feature = "recovery-sqlite")]
async fn store_with_reset_token(
    subject: &str,
    password: &str,
    now: u64,
) -> (SqlRecoveryStore, String) {
    let store = SqlRecoveryStore::connect("sqlite::memory:", keys())
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store
        .register_account(subject, EMAIL, password, now)
        .await
        .unwrap();
    consume_welcome(&store, now).await;
    store.request_password_reset(EMAIL, now + 1).await.unwrap();
    let claim = store.claim_notice(now + 2).await.unwrap().unwrap();
    let token = claim.notice().token().unwrap().expose().to_owned();
    store.complete_notice(&claim, now + 3).await.unwrap();
    (store, token)
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn failed_reset_attempts_never_block_another_members_valid_reset() {
    // Resets used to share one 60-per-minute budget charged before the token
    // check, so any unauthenticated client could block every member's reset.
    let now = 1_800_000_000;
    let (password, new) = (fixture_password(), fixture_password());
    let (store, token) = store_with_reset_token("reset-member", password.as_str(), now).await;
    let replayed = "A".repeat(43);
    let mut throttled = 0;
    for attempt in 0..100 {
        let guess = if attempt % 2 == 0 {
            replayed.clone()
        } else {
            format!(
                "{:032x}{:011x}",
                rand::random::<u128>(),
                rand::random::<u64>() >> 20
            )
        };
        match store
            .complete_password_reset(&guess, new.as_str(), now + 5)
            .await
        {
            Err(RecoveryError::InvalidAction) => {}
            Err(RecoveryError::Throttled) => throttled += 1,
            // Never format the outcome: it derives from a call that takes a password.
            Ok(()) => panic!("a guessed reset token was accepted"),
            Err(_) => panic!("unexpected reset error"),
        }
    }
    // Only the replayed token exhausts its own budget (10 failures per minute).
    assert_eq!(throttled, 40);
    store
        .complete_password_reset(&token, new.as_str(), now + 5)
        .await
        .unwrap();
    assert!(
        store
            .authenticate(EMAIL, new.as_str())
            .await
            .unwrap()
            .is_some()
    );
    // The replayed token's budget recovers after its window.
    assert_eq!(
        store
            .complete_password_reset(&replayed, new.as_str(), now + 65)
            .await,
        Err(RecoveryError::InvalidAction)
    );
    store.close().await;
}

#[cfg(feature = "recovery-sqlite")]
#[tokio::test]
async fn concurrent_attempts_with_one_valid_reset_token_are_bounded() {
    // Argon2 runs only after the token lookup. In-flight attempts hold a unit of
    // the token's budget, so a flood with one valid token cannot multiply work.
    let now = 1_800_000_000;
    let (password, new) = (fixture_password(), fixture_password());
    let (store, token) = store_with_reset_token("reset-flood", password.as_str(), now).await;
    let results = futures_util::future::join_all(
        (0..20).map(|_| store.complete_password_reset(&token, new.as_str(), now + 5)),
    )
    .await;
    let count = |expected: Result<(), RecoveryError>| {
        results.iter().filter(|result| **result == expected).count()
    };
    assert_eq!(count(Ok(())), 1);
    assert_eq!(count(Err(RecoveryError::Throttled)), 10);
    assert_eq!(count(Err(RecoveryError::InvalidAction)), 9);
    store.close().await;
}
