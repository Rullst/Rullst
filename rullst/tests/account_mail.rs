#![cfg(feature = "account-mail-sqlite")]

use rullst::{
    account_mail::{AccountMailConfig, AccountMailOutcome, deliver_next_account_mail},
    auth::recovery::{RecoverySecrets, SqlRecoveryStore},
    mail::{DeliveryPipeline, MailDriver, MailError, MailLocale, Message},
};

#[derive(Default)]
struct Capture(tokio::sync::Mutex<Vec<Message>>);

#[async_trait::async_trait]
impl MailDriver for Capture {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        self.0
            .lock()
            .await
            .push(DeliveryPipeline::prepare(message)?.into_message());
        Ok(())
    }
}

#[tokio::test]
async fn welcome_reset_and_change_notices_complete_the_real_pipeline() {
    let store = SqlRecoveryStore::connect(
        "sqlite::memory:",
        RecoverySecrets::new([4; 32], [8; 32]).unwrap(),
    )
    .await
    .unwrap();
    store.migrate().await.unwrap();
    let config = AccountMailConfig::new(
        "https://app.example",
        "https://app.example/reset",
        "Rullst Academy",
        "accounts@example.com",
        MailLocale::PtBr,
    )
    .unwrap();
    let driver = Capture::default();
    let now = chrono::Utc::now().timestamp() as u64;
    store
        .register_account("learner", "learner@example.com", "OriginalPassword12!", now)
        .await
        .unwrap();
    assert_eq!(
        deliver_next_account_mail(&store, &driver, &config)
            .await
            .unwrap(),
        AccountMailOutcome::Delivered
    );
    store
        .request_password_reset("learner@example.com", now)
        .await
        .unwrap();
    assert_eq!(
        deliver_next_account_mail(&store, &driver, &config)
            .await
            .unwrap(),
        AccountMailOutcome::Delivered
    );
    let messages = driver.0.lock().await;
    assert!(messages[0].subject.contains("Boas-vindas"));
    let reset = &messages[1];
    let url = reset
        .body_text
        .as_ref()
        .unwrap()
        .lines()
        .find(|line| line.starts_with("https://app.example/reset?token="))
        .unwrap();
    let token = url.split_once("?token=").unwrap().1;
    assert_eq!(token.len(), 43);
    assert!(reset.body_html.as_ref().unwrap().contains(url));
    assert!(!format!("{reset:?}").contains(token));
    assert!(reset.body_text.as_ref().unwrap().contains("Expira em"));
    store
        .complete_password_reset(
            token,
            "ReplacementPassword34!",
            chrono::Utc::now().timestamp() as u64,
        )
        .await
        .unwrap();
    drop(messages);
    assert_eq!(
        deliver_next_account_mail(&store, &driver, &config)
            .await
            .unwrap(),
        AccountMailOutcome::Delivered
    );
    assert_eq!(driver.0.lock().await.len(), 3);
}

struct Unavailable;
#[async_trait::async_trait]
impl MailDriver for Unavailable {
    async fn send(&self, _: &Message) -> Result<(), MailError> {
        Err(MailError::transport("fixture", "unavailable"))
    }
}

#[tokio::test]
async fn provider_failure_keeps_the_encrypted_notice_for_retry() {
    let store = SqlRecoveryStore::connect(
        "sqlite::memory:",
        RecoverySecrets::new([4; 32], [8; 32]).unwrap(),
    )
    .await
    .unwrap();
    store.migrate().await.unwrap();
    store
        .register_account(
            "learner",
            "learner@example.com",
            "OriginalPassword12!",
            chrono::Utc::now().timestamp() as u64,
        )
        .await
        .unwrap();
    let config = AccountMailConfig::new(
        "https://app.example",
        "https://app.example/reset",
        "App",
        "accounts@example.com",
        MailLocale::En,
    )
    .unwrap();
    assert_eq!(
        deliver_next_account_mail(&store, &Unavailable, &config)
            .await
            .unwrap(),
        AccountMailOutcome::RetryScheduled
    );
    let snapshot = store.outbox_snapshot().await.unwrap();
    assert_eq!(snapshot.pending, 1);
    assert_eq!(snapshot.delivered, 0);
}
