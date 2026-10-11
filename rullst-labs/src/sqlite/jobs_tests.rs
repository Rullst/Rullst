#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::test_support::*;
use super::*;
use crate::RustSource;

async fn cancelled(store: &SqliteLabs<Fixed>, name: &str) {
    submit(store, name).await;
    store
        .cancel(&Allow, &reference("alice"), &scope(), &reference(name), 1)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_validly_sealed_body_naming_another_job_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store(&directory, "labs-a").await;
    cancelled(&store, "job-1").await;
    let (mut record, content) = load(&store, "job-1").await.unwrap();
    assert!(content.is_none());
    record.view.id = reference("job-2");
    let body = store
        .key
        .seal(
            &store.record_aad(&scope(), &reference("job-1")).unwrap(),
            &serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    write(
        &path,
        "UPDATE labs_jobs SET body=? WHERE id='job-1'",
        Some(&body),
    )
    .await;
    assert!(matches!(load(&store, "job-1").await, Err(Error::Integrity)));
}

#[tokio::test]
async fn indexed_columns_must_match_the_sealed_record() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store(&directory, "labs-a").await;
    cancelled(&store, "job-1").await;
    assert!(load(&store, "job-1").await.is_ok());
    for (change, restore) in [
        (
            "UPDATE labs_jobs SET learner='bob'",
            "UPDATE labs_jobs SET learner='alice'",
        ),
        (
            "UPDATE labs_jobs SET lease_until=5",
            "UPDATE labs_jobs SET lease_until=0",
        ),
    ] {
        write(&path, change, None).await;
        assert!(
            matches!(load(&store, "job-1").await, Err(Error::Integrity)),
            "{change}"
        );
        write(&path, restore, None).await;
        assert!(load(&store, "job-1").await.is_ok());
    }
}

#[tokio::test]
async fn sealed_content_must_match_the_job_it_belongs_to() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store(&directory, "labs-a").await;
    submit(&store, "job-1").await;
    let (record, content) = load(&store, "job-1").await.unwrap();
    let payload = store.payload(&record, &content.unwrap()).unwrap();
    let source = RustSource::new("pub fn other() {}").unwrap();
    for changed in [
        Payload {
            exercise: payload.exercise.clone(),
            source,
        },
        Payload {
            exercise: exercise("sum", "v1", 580),
            source: payload.source.clone(),
        },
    ] {
        let content = store
            .key
            .seal(
                &store.job_aad(&record).unwrap(),
                &serde_json::to_vec(&changed).unwrap(),
            )
            .unwrap();
        write(
            &path,
            "UPDATE labs_jobs SET content=? WHERE id='job-1'",
            Some(&content),
        )
        .await;
        assert!(matches!(load(&store, "job-1").await, Err(Error::Integrity)));
    }
}

#[tokio::test]
async fn a_queued_job_that_used_both_attempts_expires_instead_of_running() {
    let directory = tempfile::tempdir().unwrap();
    let (store, _) = store(&directory, "labs-a").await;
    submit(&store, "job-1").await;
    let mut tx = Operation::begin(&store, None).await.unwrap();
    let (mut record, content) = store
        .load_job(&mut tx, &scope(), &reference("job-1"))
        .await
        .unwrap();
    record.attempts = 2;
    store
        .save_job(&mut tx, &record, content.as_deref(), record.view.revision)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(store.claim_next().await.unwrap().is_none());
    let (record, _) = load(&store, "job-1").await.unwrap();
    assert_eq!(record.view.state, JobState::Expired);
}

#[tokio::test]
async fn sealed_rows_are_bound_to_their_store_namespace() {
    let directory = tempfile::tempdir().unwrap();
    let (a, a_path) = store(&directory, "labs-a").await;
    let (b, b_path) = store(&directory, "labs-b").await;
    submit(&a, "job-1").await;
    submit(&b, "job-1").await;
    for (table, column) in [
        ("labs_exercises", "content"),
        ("labs_jobs", "body"),
        ("labs_jobs", "content"),
    ] {
        let select = format!("SELECT {column} FROM {table}");
        let update = format!("UPDATE {table} SET {column}=?");
        let own = read_blob(&b_path, &select).await;
        write(&b_path, &update, Some(&read_blob(&a_path, &select).await)).await;
        let mut tx = Operation::begin(&b, None).await.unwrap();
        let exercise = b
            .load_exercise(&mut tx, &scope(), &reference("sum"), &reference("v1"))
            .await;
        let job = b.load_job(&mut tx, &scope(), &reference("job-1")).await;
        assert!(
            matches!(exercise, Err(Error::Integrity)) || matches!(job, Err(Error::Integrity)),
            "{table}.{column}"
        );
        drop(tx);
        write(&b_path, &update, Some(&own)).await;
    }
}
