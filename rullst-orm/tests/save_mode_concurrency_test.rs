//! The save mode every `#[derive(Orm)]` model declares (since 13.0.0-alpha.6),
//! under concurrent requests: both read the same row, then one edits the bio
//! while the other awards XP. With a `RowSnapshot` field both changes persist;
//! with `#[orm(full_row_save)]` the later `save()` writes its whole stale row
//! back, so one change is lost, as the attribute documents.
#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{Error, Orm, RowSnapshot};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;

const DEADLINE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Orm)]
#[orm(table = "tracked_students")]
struct TrackedStudent {
    id: i32,
    bio: String,
    xp: i32,
    #[sqlx(skip)]
    snapshot: RowSnapshot,
}

#[derive(Debug, Clone, Orm)]
#[orm(table = "full_row_students", full_row_save)]
struct FullRowStudent {
    id: i32,
    bio: String,
    xp: i32,
}

async fn within<T>(step: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(DEADLINE, future)
        .await
        .unwrap_or_else(|_| panic!("{step} did not finish within {DEADLINE:?}"))
}

async fn create_table(name: &str) {
    Schema::create(name, |table: &mut Blueprint| {
        table.id();
        table.string("bio").not_null();
        table.integer("xp").not_null();
    })
    .await
    .expect("create students table");
}

/// Runs the two requests on the row `id`: each reads it, waits until the other
/// has read it too, changes its own field and saves.
async fn tracked_requests(id: i32, bio: String) {
    let barrier = Arc::new(Barrier::new(2));
    let edit = tokio::spawn({
        let barrier = Arc::clone(&barrier);
        async move {
            let mut student = TrackedStudent::find(id)
                .await?
                .ok_or(Error::RecordNotFound)?;
            within("bio request read", barrier.wait()).await;
            student.bio = bio;
            assert_eq!(student.dirty_columns(), ["bio"]);
            student.save().await
        }
    });
    let award = tokio::spawn({
        let barrier = Arc::clone(&barrier);
        async move {
            let mut student = TrackedStudent::find(id)
                .await?
                .ok_or(Error::RecordNotFound)?;
            within("XP request read", barrier.wait()).await;
            student.xp += 50;
            assert_eq!(student.dirty_columns(), ["xp"]);
            student.save().await
        }
    });
    let (edit, award) = within("tracked requests", async { tokio::join!(edit, award) }).await;
    edit.expect("bio request task").expect("save bio");
    award.expect("XP request task").expect("save XP");
}

async fn full_row_requests(id: i32) {
    let barrier = Arc::new(Barrier::new(2));
    let edit = tokio::spawn({
        let barrier = Arc::clone(&barrier);
        async move {
            let mut student = FullRowStudent::find(id)
                .await?
                .ok_or(Error::RecordNotFound)?;
            within("bio request read", barrier.wait()).await;
            student.bio = "edited bio".to_string();
            student.save().await
        }
    });
    let award = tokio::spawn({
        let barrier = Arc::clone(&barrier);
        async move {
            let mut student = FullRowStudent::find(id)
                .await?
                .ok_or(Error::RecordNotFound)?;
            within("XP request read", barrier.wait()).await;
            student.xp += 50;
            student.save().await
        }
    });
    let (edit, award) = within("full-row requests", async { tokio::join!(edit, award) }).await;
    edit.expect("bio request task").expect("save bio");
    award.expect("XP request task").expect("save XP");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_saves_of_different_fields_keep_both_only_with_a_row_snapshot() {
    Orm::init_with_options("sqlite::memory:", 1, 30)
        .await
        .expect("initialize SQLite");
    create_table("tracked_students").await;
    create_table("full_row_students").await;

    let mut tracked = TrackedStudent {
        id: 0,
        bio: "first bio".to_string(),
        xp: 10,
        snapshot: RowSnapshot::default(),
    };
    tracked.save().await.expect("insert tracked student");
    for round in 1..=5 {
        let bio = format!("edited bio {round}");
        tracked_requests(tracked.id, bio.clone()).await;
        let stored = TrackedStudent::find(tracked.id)
            .await
            .expect("read tracked student")
            .expect("tracked student");
        assert_eq!(
            (stored.bio, stored.xp),
            (bio, 10 + 50 * round),
            "round {round}: both concurrent changes persist"
        );
    }

    let mut full_row = FullRowStudent {
        id: 0,
        bio: "first bio".to_string(),
        xp: 10,
    };
    full_row.save().await.expect("insert full-row student");
    full_row_requests(full_row.id).await;
    let stored = FullRowStudent::find(full_row.id)
        .await
        .expect("read full-row student")
        .expect("full-row student");
    // The later save wrote its stale copy of the other field: exactly one of
    // the two changes survives.
    assert!(
        matches!(
            (stored.bio.as_str(), stored.xp),
            ("edited bio", 10) | ("first bio", 60)
        ),
        "a full-row save overwrites the concurrent change: {stored:?}"
    );
}
