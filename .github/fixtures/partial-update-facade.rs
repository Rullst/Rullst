use rullst::orm as rullst_orm;
use rullst_orm::{Error, FromRow, Orm};

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "facade_lessons", tenant_column = "tenant_id", auditable)]
struct Lesson {
    id: i32,
    tenant_id: String,
    title: String,
    note: Option<String>,
}

#[tokio::test]
async fn packaged_facade_partial_updates_share_the_managed_commit() {
    Orm::init_with_options("sqlite::memory:", 1, 30).await.unwrap();
    rullst_orm::schema::Schema::create("facade_lessons", |t| {
        t.id(); t.string("tenant_id").not_null(); t.string("title").not_null(); t.string("note");
    }).await.unwrap();
    rullst_orm::audit::create_audit_table().await.unwrap();
    let actor = rullst_orm::audit::AuditContext::system("archive-consumer").unwrap();
    rullst_orm::tenant::with_tenant("academy", rullst_orm::audit::with_audit_context(actor, async {
        let mut lesson = Lesson { id: 0, tenant_id: "academy".into(), title: "draft".into(), note: Some("preserve".into()) };
        lesson.save().await.unwrap();
        lesson.update_partial().title("published".into()).save().await.unwrap();
        assert_eq!(lesson.note.as_deref(), Some("preserve"));
        let id = lesson.id;
        let rollback = Orm::transaction(|handle| Box::pin(async move {
            let mut guard = handle.lock().await;
            let tx = guard.as_mut().unwrap();
            let mut row = Lesson::find_with_tx(id, tx).await?.ok_or(Error::RecordNotFound)?;
            row.update_partial().note(None).save_with_tx(tx).await?;
            assert_eq!(row.note, None);
            Err::<(), Error>(Error::Validation("archive rollback".into()))
        })).await;
        assert!(rollback.is_err());
        let durable = Lesson::find(id).await.unwrap().unwrap();
        assert_eq!(durable.title, "published");
        assert_eq!(durable.note.as_deref(), Some("preserve"));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rullst_audits WHERE model_type = 'facade_lessons'")
            .fetch_one(Orm::pool().unwrap()).await.unwrap();
        assert_eq!(count, 2);
    })).await;
}
