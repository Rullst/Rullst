#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::test_support::*;
use super::*;

async fn load(store: &SqliteLabs<Fixed>) -> Result<(Exercise, bool), Error> {
    let mut tx = Operation::begin(store, None).await?;
    let loaded = store
        .load_exercise(&mut tx, &scope(), &reference("sum"), &reference("v1"))
        .await?;
    tx.commit().await?;
    Ok(loaded)
}

#[tokio::test]
async fn a_validly_sealed_snapshot_of_another_exercise_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store(&directory, "labs-a").await;
    for other in [exercise("other", "v1", 579), exercise("sum", "v2", 579)] {
        let sealed = store
            .key
            .seal(
                &store
                    .exercise_aad(&scope(), &reference("sum"), &reference("v1"))
                    .unwrap(),
                &serde_json::to_vec(&RegisteredExercise {
                    exercise: other.clone(),
                    enabled: true,
                })
                .unwrap(),
            )
            .unwrap();
        write(&path, "UPDATE labs_exercises SET content=?", Some(&sealed)).await;
        write(
            &path,
            &format!(
                "UPDATE labs_exercises SET digest='{}'",
                other.digest().unwrap().as_str()
            ),
            None,
        )
        .await;
        assert!(matches!(load(&store).await, Err(Error::Integrity)));
    }
}

#[tokio::test]
async fn indexed_exercise_columns_must_match_the_sealed_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let (store, path) = store(&directory, "labs-a").await;
    store
        .set_exercise_enabled(
            &Allow,
            &reference("teacher"),
            &scope(),
            &reference("sum"),
            &reference("v1"),
            false,
        )
        .await
        .unwrap();
    assert!(matches!(load(&store).await, Ok((_, false))));
    let digest = format!(
        "UPDATE labs_exercises SET digest='{}'",
        ContentHash::of(b"another exercise").as_str()
    );
    let original = format!(
        "UPDATE labs_exercises SET digest='{}'",
        exercise("sum", "v1", 579).digest().unwrap().as_str()
    );
    for (change, restore) in [
        (digest.as_str(), original.as_str()),
        (
            "UPDATE labs_exercises SET enabled=2",
            "UPDATE labs_exercises SET enabled=0",
        ),
        (
            "UPDATE labs_exercises SET enabled=1",
            "UPDATE labs_exercises SET enabled=0",
        ),
    ] {
        write(
            &path,
            &format!("PRAGMA ignore_check_constraints = ON; {change}"),
            None,
        )
        .await;
        assert!(
            matches!(load(&store).await, Err(Error::Integrity)),
            "{change}"
        );
        write(&path, restore, None).await;
        assert!(matches!(load(&store).await, Ok((_, false))));
    }
}
