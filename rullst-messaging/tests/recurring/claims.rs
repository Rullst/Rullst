use super::support::*;

/// Returns the current time and then advances by `step` on every read, so a
/// deadline can pass between reads inside one store operation.
#[derive(Clone)]
struct SteppingClock {
    now: Arc<AtomicI64>,
    step: Arc<AtomicI64>,
}

impl SteppingClock {
    fn set(&self, now: i64, step: i64) {
        self.step.store(step, Ordering::SeqCst);
        self.now.store(now, Ordering::SeqCst);
    }
}

impl Clock for SteppingClock {
    fn now_millis(&self) -> rullst_messaging::Result<i64> {
        Ok(self
            .now
            .fetch_add(self.step.load(Ordering::SeqCst), Ordering::SeqCst))
    }
}

/// One occurrence at the end of its delivery window neither fails the claim
/// nor orphans the other leases, and a committed operator retry reports success.
pub async fn run(url: &str) {
    let namespace = unique();
    let start = ManualClock::new().now_millis().unwrap();
    let clock = SteppingClock {
        now: Arc::new(AtomicI64::new(start)),
        step: Arc::new(AtomicI64::new(0)),
    };
    let store = PostgresRecurringStore::initialize(url, config(&namespace), keys(), clock.clone())
        .await
        .unwrap();
    let schedule = RecurringDefinition::new(
        "minutely",
        "* * * * *",
        start - 60_000,
        MissedRunPolicy::CatchUp,
        ScheduledMessage::new("scheduled", "account.reminder", b"{}".to_vec()).unwrap(),
    )
    .unwrap();
    store.create(schedule).await.unwrap();
    let closing = store.tick(1).await.unwrap().remove(0);
    clock.set(start + 60_000, 0);
    let fresh = store.tick(1).await.unwrap().remove(0);
    assert!(closing.expires_at_ms() < fresh.expires_at_ms());

    // The claim samples time twice before committing: the closing occurrence
    // is still claimable at the first sample and expired by the commit.
    clock.set(closing.expires_at_ms() - 2, 1);
    let leases = store.claim(10).await.unwrap();
    assert_eq!(leases.len(), 1);
    assert_eq!(leases[0].metadata().id(), fresh.id());

    clock.set(closing.expires_at_ms() + 10, 0);
    let rows = store.occurrences(None, 10).await.unwrap();
    let closed = rows.iter().find(|row| row.id() == closing.id()).unwrap();
    assert_eq!(closed.state(), OccurrenceState::DeadLetter);

    // An operator retry whose reset commits just before the window closes.
    let admin = admin(url).await;
    sqlx::query(
        "UPDATE rullst_recurring_occurrences SET state='dead',lease_hash=NULL,
        lease_until=NULL,terminal_at=$1 WHERE namespace=$2 AND id=$3",
    )
    .bind(closing.expires_at_ms() + 10)
    .bind(&namespace)
    .bind(fresh.id())
    .execute(&admin)
    .await
    .unwrap();
    clock.set(fresh.expires_at_ms() - 3, 1);
    assert_eq!(store.retry_failed(fresh.id()).await, Ok(()));
    admin.close().await;
    store.close().await;
}
