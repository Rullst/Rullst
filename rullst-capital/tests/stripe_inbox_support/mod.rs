#![allow(dead_code)]

use ring::hmac;
use rullst_capital::{
    SqlStripeEventInbox, SqlWebhookBackend, StripeInboxError as Error,
    StripeInboxOutcome as Outcome, StripeInboxScope, StripeProvider, StripeSubscriptionEvent,
};
use rullst_orm::{RullstPool, RullstPoolOptions, sqlx};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn supports_backend(backend: SqlWebhookBackend) -> bool {
    use std::any::TypeId;
    let primary = TypeId::of::<rullst_orm::database::RullstDatabase>();
    primary == TypeId::of::<sqlx::Any>()
        || primary
            == match backend {
                SqlWebhookBackend::Postgres => TypeId::of::<sqlx::Postgres>(),
                SqlWebhookBackend::Mysql => TypeId::of::<sqlx::MySql>(),
                SqlWebhookBackend::Sqlite => TypeId::of::<sqlx::Sqlite>(),
                _ => panic!("unsupported test backend"),
            }
}

pub async fn pool(url: &str) -> RullstPool {
    sqlx::any::install_default_drivers();
    RullstPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(url)
        .await
        .expect("inbox test database")
}

pub fn snapshot(id: &str) -> Value {
    json!({"id":id,"object":"event","type":"customer.subscription.updated",
    "api_version":"2025-03-31.basil","created":1750000000,"livemode":false,
    "data":{"object":{"id":"sub_inbox","object":"subscription",
        "customer":"cus_inbox","status":"active","livemode":false,
        "metadata":{"rullst_owner_reference":"owner_inbox"},
        "items":{"has_more":false,"data":[{"price":{"id":"price_inbox","type":"recurring"}}]}
    }}})
}

pub fn verify(value: &Value, mock: bool) -> StripeSubscriptionEvent {
    let payload = serde_json::to_vec(value).unwrap();
    let now = chrono::Utc::now().timestamp();
    let mut context = hmac::Context::with_key(&hmac::Key::new(hmac::HMAC_SHA256, b"whsec_inbox"));
    context.update(format!("{now}.").as_bytes());
    context.update(&payload);
    let headers = HashMap::from([(
        "stripe-signature".into(),
        if mock {
            "mock_inbox".into()
        } else {
            format!("t={now},v1={}", hex::encode(context.sign()))
        },
    )]);
    let secret = if mock { "mock_inbox" } else { "whsec_inbox" };
    StripeProvider::new("sk_test_fixture", secret)
        .verify_subscription_event(&payload, &headers)
        .unwrap()
}

pub fn inbox(
    pool: &RullstPool,
    backend: SqlWebhookBackend,
    namespace: &str,
    capacity: usize,
) -> SqlStripeEventInbox {
    SqlStripeEventInbox::new(
        pool.clone(),
        backend,
        StripeInboxScope::platform(namespace, "acct_inbox", false).unwrap(),
        capacity,
    )
    .unwrap()
}

fn insert_sql(backend: SqlWebhookBackend) -> &'static str {
    match backend {
        SqlWebhookBackend::Postgres => "INSERT INTO inbox_domain_effects (event_name) VALUES ($1)",
        _ => "INSERT INTO inbox_domain_effects (event_name) VALUES (?)",
    }
}

async fn count(pool: &RullstPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM inbox_domain_effects")
        .fetch_one(pool)
        .await
        .unwrap()
}

pub async fn exercise(pool: &RullstPool, backend: SqlWebhookBackend) {
    sqlx::query("CREATE TABLE inbox_domain_effects (event_name VARCHAR(200) NOT NULL PRIMARY KEY)")
        .execute(pool)
        .await
        .unwrap();
    let store = inbox(pool, backend, "atomic", 8);
    store.prepare_schema().await.unwrap();
    let event = verify(&snapshot("evt_atomic"), false);
    let insert = insert_sql(backend);
    let failed = store
        .process(&event, move |tx, event| {
            Box::pin(async move {
                sqlx::query(insert)
                    .bind(event.event_id())
                    .execute(&mut **tx)
                    .await
                    .unwrap();
                Err(Error::MutationRejected)
            })
        })
        .await;
    assert_eq!(failed, Err(Error::MutationRejected));
    assert_eq!(
        count(pool).await,
        0,
        "failed mutation must roll back its SQL"
    );
    let first = store
        .process(&event, move |tx, event| {
            Box::pin(async move {
                sqlx::query(insert)
                    .bind(event.event_id())
                    .execute(&mut **tx)
                    .await
                    .unwrap();
                Ok(Outcome::Applied)
            })
        })
        .await
        .unwrap();
    assert!(!first.is_duplicate());
    assert_eq!(first.outcome(), Outcome::Applied);
    assert_eq!(count(pool).await, 1);
    let restarted = inbox(pool, backend, "atomic", 8);
    restarted.prepare_schema().await.unwrap();
    let replay = restarted
        .process(&event, |_, _| {
            Box::pin(async { panic!("committed event must not rerun") })
        })
        .await
        .unwrap();
    assert!(replay.is_duplicate());
    assert_eq!(replay.outcome(), first.outcome());
    let mut delivery = snapshot("evt_atomic");
    delivery["pending_webhooks"] = json!(3);
    delivery["data"]["object"]["customer_email"] = json!("changed@example.com");
    assert!(
        restarted
            .process(&verify(&delivery, false), |_, _| Box::pin(async {
                panic!("delivery changes must not rerun")
            }))
            .await
            .unwrap()
            .is_duplicate()
    );
    delivery["data"]["object"]["status"] = json!("past_due");
    assert_eq!(
        restarted
            .process(&verify(&delivery, false), |_, _| Box::pin(async {
                panic!("conflicting event must not run")
            }))
            .await,
        Err(Error::EventConflict)
    );

    // Concurrent contenders use independent pool connections and store instances.
    let event = verify(&snapshot("evt_concurrent"), false);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let store = inbox(pool, backend, "atomic", 8);
        let event = event.clone();
        tasks.spawn(async move {
            store
                .process(&event, move |tx, event| {
                    Box::pin(async move {
                        sqlx::query(insert)
                            .bind(event.event_id())
                            .execute(&mut **tx)
                            .await
                            .unwrap();
                        Ok(Outcome::Applied)
                    })
                })
                .await
                .unwrap()
        });
    }
    let mut fresh = 0;
    let mut duplicates = 0;
    while let Some(result) = tasks.join_next().await {
        if result.unwrap().is_duplicate() {
            duplicates += 1;
        } else {
            fresh += 1;
        }
    }
    assert_eq!((fresh, duplicates), (1, 7));
    assert_eq!(count(pool).await, 2);

    // A task canceled after domain SQL but before commit must leave neither row.
    let (signal, started) = tokio::sync::oneshot::channel();
    let cancelled_store = inbox(pool, backend, "atomic", 8);
    let event = verify(&snapshot("evt_cancelled"), false);
    let retry = event.clone();
    let task = tokio::spawn(async move {
        cancelled_store
            .process(&event, move |tx, event| {
                Box::pin(async move {
                    sqlx::query(insert)
                        .bind(event.event_id())
                        .execute(&mut **tx)
                        .await
                        .unwrap();
                    signal.send(()).unwrap();
                    std::future::pending::<()>().await;
                    Ok(Outcome::Applied)
                })
            })
            .await
    });
    started.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let retried = store
        .process(&retry, move |tx, event| {
            Box::pin(async move {
                sqlx::query(insert)
                    .bind(event.event_id())
                    .execute(&mut **tx)
                    .await
                    .unwrap();
                Ok(Outcome::Applied)
            })
        })
        .await
        .unwrap();
    assert!(!retried.is_duplicate());
    assert_eq!(count(pool).await, 3);

    let bounded = inbox(pool, backend, "bounded", 1);
    bounded.prepare_schema().await.unwrap();
    let ignored = bounded
        .process(&retry, |_, _| Box::pin(async { Ok(Outcome::Ignored) }))
        .await
        .unwrap();
    assert_eq!(ignored.outcome(), Outcome::Ignored);
    assert!(
        bounded
            .process(&retry, |_, _| Box::pin(async {
                panic!("full store must retain exact retry")
            }))
            .await
            .unwrap()
            .is_duplicate()
    );
    let extra = verify(&snapshot("evt_full"), false);
    assert_eq!(
        bounded
            .process(&extra, |_, _| Box::pin(async {
                panic!("full store must not mutate")
            }))
            .await,
        Err(Error::CapacityExhausted)
    );
    let drift = inbox(pool, backend, "bounded", 2);
    assert_eq!(
        drift.prepare_schema().await,
        Err(Error::ConfigurationMismatch)
    );
    assert_eq!(
        drift
            .process(&extra, |_, _| Box::pin(async {
                panic!("profile drift must not mutate")
            }))
            .await,
        Err(Error::ConfigurationMismatch)
    );

    // Account, application and mode namespaces cannot share admission records.
    for scope in [
        StripeInboxScope::platform("another_application", "acct_inbox", false).unwrap(),
        StripeInboxScope::platform("atomic", "acct_other", false).unwrap(),
    ] {
        let isolated = SqlStripeEventInbox::new(pool.clone(), backend, scope, 1).unwrap();
        isolated.prepare_schema().await.unwrap();
        assert!(
            !isolated
                .process(&retry, |_, _| Box::pin(async { Ok(Outcome::Ignored) }))
                .await
                .unwrap()
                .is_duplicate()
        );
    }
    scope_failures(pool, backend).await;
}

async fn scope_failures(pool: &RullstPool, backend: SqlWebhookBackend) {
    // No schema/profile exists for this scope: these denials must happen before SQL.
    let store = inbox(pool, backend, "scope_without_schema", 1);
    let event = snapshot("evt_scope");
    assert_eq!(
        store
            .process(&verify(&event, true), |_, _| Box::pin(async {
                panic!("mock")
            }))
            .await,
        Err(Error::MockEvent)
    );
    let mut wrong = event.clone();
    wrong["livemode"] = json!(true);
    wrong["data"]["object"]["livemode"] = json!(true);
    assert_eq!(
        store
            .process(&verify(&wrong, false), |_, _| Box::pin(async {
                panic!("mode")
            }))
            .await,
        Err(Error::ScopeMismatch)
    );
    wrong = event;
    wrong["account"] = json!("acct_connected");
    let connected_event = verify(&wrong, false);
    assert_eq!(
        store
            .process(&connected_event, |_, _| Box::pin(async {
                panic!("connected account")
            }))
            .await,
        Err(Error::ScopeMismatch)
    );
    let connected = SqlStripeEventInbox::new(
        pool.clone(),
        backend,
        StripeInboxScope::connected("connected", "acct_connected", false).unwrap(),
        1,
    )
    .unwrap();
    connected.prepare_schema().await.unwrap();
    connected
        .process(&connected_event, |_, _| {
            Box::pin(async { Ok(Outcome::Ignored) })
        })
        .await
        .unwrap();
    wrong["account"] = json!("acct_intruder");
    assert_eq!(
        connected
            .process(&verify(&wrong, false), |_, _| Box::pin(async {
                panic!("wrong account")
            }))
            .await,
        Err(Error::ScopeMismatch)
    );
}
