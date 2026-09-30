use super::support::*;

async fn recorded(admin: &sqlx::PgPool, namespace: &str) -> i64 {
    sqlx::query_scalar("SELECT last_now FROM rullst_recurring_control WHERE namespace=$1")
        .bind(namespace)
        .fetch_one(admin)
        .await
        .unwrap()
}

/// Independent instances share one namespace but not one clock.
pub async fn run(url: &str) {
    let namespace = unique();
    let admin = admin(url).await;
    let leading = ManualClock::new();
    let trailing = ManualClock::new();
    trailing.advance(-20);
    let leader = open(url, &namespace, &leading).await;
    let follower =
        PostgresRecurringStore::connect(url, config(&namespace), keys(), trailing.clone())
            .await
            .unwrap();
    leader
        .create(definition("one", &leading, MissedRunPolicy::CatchUp))
        .await
        .unwrap();
    let high_water = recorded(&admin, &namespace).await;

    // A follower milliseconds or seconds behind adopts the recorded time.
    assert_eq!(follower.tick(1).await.unwrap().len(), 1);
    trailing.advance(-3_980);
    assert_eq!(follower.schedules(None, 1).await.unwrap().len(), 1);
    assert_eq!(follower.claim(1).await.unwrap().len(), 1);
    assert_eq!(recorded(&admin, &namespace).await, high_water);

    // The leader keeps advancing shared time; it never moves backwards.
    leading.advance(10);
    leader.schedules(None, 1).await.unwrap();
    assert_eq!(recorded(&admin, &namespace).await, high_water + 10);

    // A regression beyond the tolerance still fails closed.
    trailing.advance(-1_100);
    assert_eq!(
        follower.schedules(None, 1).await,
        Err(RecurringError::Clock)
    );
    assert_eq!(recorded(&admin, &namespace).await, high_water + 10);
    follower.close().await;
    leader.close().await;
}
