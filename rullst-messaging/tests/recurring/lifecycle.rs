use super::support::*;
use sqlx::Row;

pub async fn run(url: &str) {
    let namespace = unique();
    let clock = ManualClock::new();
    let (left, right) = tokio::join!(open(url, &namespace, &clock), open(url, &namespace, &clock));
    let def = definition("hourly", &clock, MissedRunPolicy::CatchUp);
    let (a, b) = tokio::join!(left.create(def.clone()), right.create(def.clone()));
    assert_eq!(a.unwrap(), b.unwrap());
    assert_eq!(
        left.create(definition("hourly", &clock, MissedRunPolicy::Coalesce))
            .await,
        Err(RecurringError::Conflict)
    );
    let (a, b) = tokio::join!(left.tick(10), right.tick(10));
    assert_eq!(a.unwrap().len() + b.unwrap().len(), 1);
    let (a, b) = tokio::join!(left.claim(10), right.claim(10));
    let mut leases = a.unwrap();
    leases.extend(b.unwrap());
    assert_eq!(leases.len(), 1);
    let old = leases.remove(0);
    clock.advance(2001);
    let fresh = right.claim(1).await.unwrap().remove(0);
    let memory =
        InMemoryBroker::with_clock(BrokerConfig::try_new("fixture").unwrap(), clock.clone());
    assert!(matches!(
        left.relay(&old, &memory).await,
        Err(RecurringRelayError::BeforePublication(
            RecurringError::InvalidLease
        ))
    ));
    let receipt = right.relay(&fresh, &memory).await.unwrap();
    assert!(receipt.occurrence_acknowledged());
    assert!(right.relay(&fresh, &memory).await.is_err());
    let admin = admin(url).await;
    let content: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT content FROM rullst_recurring_occurrences WHERE namespace=$1")
            .bind(&namespace)
            .fetch_one(&admin)
            .await
            .unwrap();
    assert!(content.is_none());
    let bytes: Vec<u8> =
        sqlx::query_scalar("SELECT content FROM rullst_recurring_definitions WHERE namespace=$1")
            .bind(&namespace)
            .fetch_one(&admin)
            .await
            .unwrap();
    assert!(!bytes.windows(7).any(|w| w == b"PRIVATE"));
    clock.advance(180_000);
    assert_eq!(left.tick(2).await.unwrap().len(), 2);
    assert_eq!(right.tick(10).await.unwrap().len(), 1);
    let pending = left.claim(1).await.unwrap().remove(0);
    right.cancel("hourly").await.unwrap();
    assert!(left.relay(&pending, &memory).await.is_err());
    assert!(left.create(def).await.unwrap().is_cancelled());
    assert!(left.tick(100).await.unwrap().is_empty());
    assert!(left.claim(100).await.unwrap().is_empty());
    let rows = left.occurrences(None, 100).await.unwrap();
    assert_eq!(
        rows.iter()
            .filter(|r| r.state() == OccurrenceState::Cancelled)
            .count(),
        3
    );
    let first = left.occurrences(None, 1).await.unwrap();
    assert_eq!(
        left.occurrences(Some(first[0].id()), 100)
            .await
            .unwrap()
            .len(),
        3
    );
    clock.advance(1);
    assert_eq!(
        left.purge_terminal(clock.now_millis().unwrap(), 100)
            .await
            .unwrap(),
        4
    );
    assert!(left.retry_failed(pending.metadata().id()).await.is_err());
    let other = open(url, &unique(), &clock).await;
    assert!(other.relay(&pending, &memory).await.is_err());
    other.close().await;
    // Coalescing advances past now even after multiple missed instants.
    let coalesce = open(url, &unique(), &clock).await;
    coalesce
        .create(definition("one", &clock, MissedRunPolicy::Coalesce))
        .await
        .unwrap();
    clock.advance(600_000);
    assert_eq!(coalesce.tick(100).await.unwrap().len(), 1);
    assert!(coalesce.tick(100).await.unwrap().is_empty());
    // Real data corruption must authenticate before schedule advancement.
    let generation = coalesce.schedules(None, 1).await.unwrap()[0]
        .generation()
        .to_owned();
    sqlx::query("UPDATE rullst_recurring_definitions SET content=$1 WHERE generation=$2")
        .bind(bytes)
        .bind(generation)
        .execute(&admin)
        .await
        .unwrap();
    clock.advance(60_000);
    assert_eq!(coalesce.tick(1).await, Err(RecurringError::Encryption));
    let row = sqlx::query("SELECT COUNT(*) AS count FROM rullst_recurring_control")
        .fetch_one(&admin)
        .await
        .unwrap();
    assert!(row.get::<i64, _>("count") >= 3);
    coalesce.close().await;
    left.close().await;
    right.close().await;
    admin.close().await;
}
