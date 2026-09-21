#![allow(clippy::expect_used, clippy::unwrap_used)]

use rullst_orm::audit::{AuditContext, with_audit_context};
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::tenant::with_tenant;
use rullst_orm::{Error, FromRow, ModelCommittedEvent, Orm, Policy};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(
    table = "partial_lifecycle_lessons",
    tenant_column = "tenant_id",
    auditable,
    policy = "LessonPolicy",
    before_save = "validate",
    after_save = "validate_saved"
)]
struct Lesson {
    id: i32,
    tenant_id: String,
    title: String,
    note: Option<String>,
    stage: i32,
}
impl Lesson {
    async fn validate(&mut self) -> Result<(), Error> {
        if self.title == "hook-denied" {
            return Err(Error::Validation("fixture before hook".into()));
        }
        self.title = self.title.trim().to_owned();
        Ok(())
    }
    async fn validate_saved(&mut self) -> Result<(), Error> {
        if self.title == "cancel-in-hook" {
            CANCEL_REACHED.notify_one();
            std::future::pending::<()>().await;
        }
        if self.title == "after-denied" {
            return Err(Error::Validation("fixture after hook".into()));
        }
        Ok(())
    }
}
static CANCEL_REACHED: tokio::sync::Notify = tokio::sync::Notify::const_new();

struct LessonPolicy;
#[rullst_orm::async_trait]
impl Policy<Lesson> for LessonPolicy {
    async fn can_update(model: &Lesson) -> Result<bool, Error> {
        if model.title == "reentrant" {
            let _ = Lesson::find(model.id).await?;
        }
        Ok(model.title != "denied")
    }
}
struct Events(Arc<Mutex<Vec<String>>>);
#[rullst_orm::async_trait]
impl LessonObserver for Events {
    async fn updated(&self, model: &Lesson) -> Result<(), Error> {
        if model.title == "observer-denied" {
            return Err(Error::Validation("fixture observer".into()));
        }
        Ok(())
    }
    async fn committed(&self, event: &ModelCommittedEvent) -> Result<(), Error> {
        self.0.lock().unwrap().push(event.payload.clone());
        if event.payload.contains("effect-failed") {
            return Err(Error::Internal("fixture committed observer".into()));
        }
        Ok(())
    }
}

fn statement(source: &str) -> String {
    if Orm::driver().unwrap() == "postgres" {
        rullst_orm::replace_placeholders(source)
    } else {
        source.to_owned()
    }
}
async fn audit_count() -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM rullst_audits WHERE model_type = 'partial_lifecycle_lessons'",
    )
    .fetch_one(Orm::pool().unwrap())
    .await
    .unwrap()
}
async fn current(id: i32) -> Lesson {
    Lesson::find(id).await.unwrap().unwrap()
}
fn context() -> AuditContext {
    AuditContext::system("partial-lifecycle-fixture").unwrap()
}

/// Same logical journey is invoked after initialization by SQLite, PostgreSQL and MySQL matrices.
pub async fn exercise() {
    Schema::create("partial_lifecycle_lessons", |t: &mut Blueprint| {
        t.id();
        t.string("tenant_id").not_null();
        t.string("title").not_null();
        t.string("note");
        t.integer("stage").not_null();
    })
    .await
    .unwrap();
    sqlx::query(if Orm::driver().unwrap() == "mysql" {
        "CREATE UNIQUE INDEX partial_lifecycle_title ON partial_lifecycle_lessons(title(191))"
    } else {
        "CREATE UNIQUE INDEX partial_lifecycle_title ON partial_lifecycle_lessons(title)"
    })
    .execute(Orm::pool().unwrap())
    .await
    .unwrap();
    rullst_orm::audit::create_audit_table().await.unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    Lesson::observe(Arc::new(Events(events.clone())));
    let foreign_id = with_tenant(
        "tenant-b",
        with_audit_context(context(), async {
            let mut other = Lesson {
                id: 0,
                tenant_id: "tenant-b".into(),
                title: "foreign".into(),
                note: None,
                stage: 1,
            };
            other.save().await.unwrap();
            other.id
        }),
    )
    .await;
    with_tenant("tenant-a", async {
        let mut lesson = Lesson {
            id: 0,
            tenant_id: "tenant-a".into(),
            title: "original".into(),
            note: Some("initial".into()),
            stage: 1,
        };
        with_audit_context(context(), lesson.save()).await.unwrap();
        let calls = events.lock().unwrap().len();
        let audits = audit_count().await;
        assert!(
            lesson
                .update_partial()
                .title("missing-audit-context".into())
                .save()
                .await
                .is_err()
        );
        assert_eq!(lesson.title, "original");
        assert_eq!(current(lesson.id).await.title, "original");
        assert_eq!(events.lock().unwrap().len(), calls);
        assert_eq!(audit_count().await, audits);
        with_audit_context(context(), journey(lesson, foreign_id, events.clone())).await;
    })
    .await;
}

async fn journey(mut lesson: Lesson, foreign_id: i32, events: Arc<Mutex<Vec<String>>>) {
    let calls = events.lock().unwrap().len();
    let audits = audit_count().await;
    // Rejected policy, before/after hooks, observer and a real unique constraint all preserve state.
    for title in [
        "denied",
        "hook-denied",
        "after-denied",
        "observer-denied",
        "foreign",
        "reentrant",
    ] {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            lesson.update_partial().title(title.into()).save(),
        )
        .await
        .expect("callback must not deadlock");
        assert!(result.is_err(), "unexpected acceptance of {title}");
        assert_eq!(lesson.title, "original");
        assert_eq!(current(lesson.id).await.title, "original");
        assert_eq!(audit_count().await, audits);
        assert_eq!(events.lock().unwrap().len(), calls);
    }
    {
        let pending = lesson
            .update_partial()
            .title("cancel-in-hook".into())
            .save();
        tokio::pin!(pending);
        tokio::select! {
            _ = &mut pending => panic!("fixture hook must remain pending"),
            result = tokio::time::timeout(std::time::Duration::from_secs(3), CANCEL_REACHED.notified()) => result.unwrap(),
        }
        // Dropping the in-flight write must roll back its SQLx transaction/savepoints.
    }
    assert_eq!(lesson.title, "original");
    assert_eq!(current(lesson.id).await.title, "original");
    assert_eq!(events.lock().unwrap().len(), calls);
    assert_eq!(audit_count().await, audits);
    let mut outside = lesson.clone();
    outside.tenant_id = "tenant-b".into();
    assert!(
        outside
            .update_partial()
            .title("outside".into())
            .save()
            .await
            .is_err()
    );
    assert_eq!(outside.title, "original");
    let mut forged = lesson.clone();
    forged.id = foreign_id;
    assert!(matches!(
        forged.update_partial().title("forged".into()).save().await,
        Err(Error::RecordNotFound)
    ));
    let mut missing = lesson.clone();
    missing.id = i32::MAX;
    assert!(matches!(
        missing
            .update_partial()
            .title("missing".into())
            .save()
            .await,
        Err(Error::RecordNotFound)
    ));
    lesson.update_partial().save().await.unwrap();
    assert_eq!(audit_count().await, audits);
    assert_eq!(events.lock().unwrap().len(), calls);

    // An unrelated server-side change and an unsaved client edit must not be overwritten.
    let sql = statement("UPDATE partial_lifecycle_lessons SET note = ? WHERE id = ?");
    sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind("fresh database note")
        .bind(lesson.id)
        .execute(Orm::pool().unwrap())
        .await
        .unwrap();
    lesson.stage = 999; // Not part of the submitted patch.
    lesson
        .update_partial()
        .title("  published  ".into())
        .save()
        .await
        .unwrap();
    assert_eq!(
        lesson.title, "published",
        "before hook runs on the merged candidate"
    );
    assert_eq!(lesson.stage, 1);
    assert_eq!(lesson.note.as_deref(), Some("fresh database note"));
    assert_eq!(events.lock().unwrap().len(), calls + 1);
    assert_eq!(audit_count().await, audits + 1);
    let sql = statement(
        "SELECT old_values, new_values FROM rullst_audits WHERE model_type = 'partial_lifecycle_lessons' AND model_id = ? AND event = 'updated' ORDER BY id DESC LIMIT 1",
    );
    let patch: (Option<String>, Option<String>) = sqlx::query_as(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(lesson.id)
        .fetch_one(Orm::pool().unwrap())
        .await
        .unwrap();
    assert!(patch.0.unwrap().contains("original"));
    assert!(patch.1.unwrap().contains("published"));
    assert_eq!(current(lesson.id).await.stage, 1);
    lesson.update_partial().note(None).save().await.unwrap();
    assert_eq!(current(lesson.id).await.note, None);
    managed_transactions(&mut lesson, &events).await;

    let before_effect = audit_count().await;
    assert!(matches!(
        lesson
            .update_partial()
            .title("effect-failed".into())
            .save()
            .await,
        Err(Error::PostCommit(_))
    ));
    assert_eq!(lesson.title, "effect-failed");
    assert_eq!(current(lesson.id).await.title, "effect-failed");
    assert_eq!(audit_count().await, before_effect + 1);
    concurrent_patches(lesson.id).await;
}

async fn managed_transactions(lesson: &mut Lesson, events: &Arc<Mutex<Vec<String>>>) {
    let id = lesson.id;
    let before = audit_count().await;
    let calls = events.lock().unwrap().len();
    let pending_events = events.clone();
    let rollback = Orm::transaction(|_| {
        Box::pin(async move {
            let mut row = Lesson::find(id).await?.ok_or(Error::RecordNotFound)?;
            row.update_partial()
                .title("tentative".into())
                .save()
                .await?;
            assert_eq!(row.title, "tentative");
            assert_eq!(Lesson::find(id).await?.unwrap().title, "tentative");
            assert_eq!(pending_events.lock().unwrap().len(), calls);
            Err::<(), Error>(Error::Validation("fixture outer rollback".into()))
        })
    })
    .await;
    assert!(rollback.is_err());
    assert_eq!(current(id).await.title, "published");
    assert_eq!(events.lock().unwrap().len(), calls);
    assert_eq!(audit_count().await, before);

    let pending_events = events.clone();
    *lesson = Orm::transaction(|transaction| {
        Box::pin(async move {
            let mut guard = transaction.lock().await;
            let tx = guard.as_mut().unwrap();
            let mut row = Lesson::find_with_tx(id, tx)
                .await?
                .ok_or(Error::RecordNotFound)?;
            assert!(
                row.update_partial()
                    .title("after-denied".into())
                    .save_with_tx(tx)
                    .await
                    .is_err()
            );
            assert_eq!(row.title, "published");
            assert_eq!(
                Lesson::find_with_tx(id, tx).await?.unwrap().title,
                "published"
            );
            row.update_partial()
                .title("explicit".into())
                .save_with_tx(tx)
                .await?;
            assert_eq!(pending_events.lock().unwrap().len(), calls);
            Ok::<_, Error>(row)
        })
    })
    .await
    .unwrap();
    assert_eq!(lesson.title, "explicit");
    assert_eq!(events.lock().unwrap().len(), calls + 1);
    assert_eq!(
        audit_count().await,
        before + 1,
        "failed savepoint must not leave an audit row"
    );
}

async fn concurrent_patches(id: i32) {
    current(id)
        .await
        .update_partial()
        .title("concurrent-start".into())
        .save()
        .await
        .unwrap();
    let mut first = current(id).await;
    let mut second = current(id).await;
    // Both handles were loaded before either write. The locked merge must retain both fields.
    let (one, two) = tokio::join!(
        first
            .update_partial()
            .title("concurrent-title".into())
            .save(),
        second.update_partial().stage(2).save(),
    );
    let known_rollback = |result: Result<(), Error>| match result {
        Ok(()) => false,
        Err(Error::DatabaseError(message))
            if Orm::driver().unwrap() == "sqlite"
                && (message.contains("locked") || message.contains("deadlocked")) =>
        {
            true
        }
        other => panic!("unexpected concurrent patch result: {other:?}"),
    };
    let retry_title = known_rollback(one);
    let retry_stage = known_rollback(two);
    let stored = current(id).await;
    if retry_title {
        assert_eq!(first.title, "concurrent-start");
        assert_eq!(stored.title, "concurrent-start");
    }
    if retry_stage {
        assert_eq!(second.stage, 1);
        assert_eq!(stored.stage, 1);
    }
    // Explicit fresh user-level retries after these confirmed SQLite rollbacks.
    // The production method never automatically replays an ambiguous operation.
    if retry_title {
        first
            .update_partial()
            .title("concurrent-title".into())
            .save()
            .await
            .unwrap();
    }
    if retry_stage {
        second.update_partial().stage(2).save().await.unwrap();
    }
    let row = current(id).await;
    assert_eq!(row.title, "concurrent-title");
    assert_eq!(row.stage, 2);
}
