#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use rullst_orm::{Error, Orm, with_tenant};
#[path = "builder_safety_audit/support.rs"]
mod support;
use support::*;

async fn safe_select_rejects_expressions_and_malformed_identifiers() {
    let _guard = database().await;
    for column in [
        "(SELECT secret FROM audit_secrets LIMIT 1) AS name",
        "name; --",
        "name AS name",
        "name, id",
        "",
        "*",
        "audit_records..name",
        "name\0",
    ] {
        validation_error(AuditRecord::query().select(&["id", column]).get().await);
    }
    let rows = AuditRecord::query()
        .select(&["audit_records.id", "audit_records.name"])
        .order_by("id")
        .get()
        .await
        .expect("qualified safe identifiers remain supported");
    assert_eq!(rows[0].name, "alpha");
    assert_eq!(rows.len(), 2);
    let raw = AuditRecord::query()
        .select_raw("id, upper(name) AS name")
        .order_by("id")
        .get()
        .await
        .expect("explicit raw expression remains available");
    assert_eq!(raw[0].name, "ALPHA");
}

async fn safe_pluck_rejects_expressions_before_executing_sql() {
    let _guard = database().await;
    validation_error(
        AuditRecord::query()
            .pluck_string("(SELECT secret FROM audit_secrets LIMIT 1)")
            .await,
    );
    validation_error(AuditRecord::query().pluck_i32("id + 100").await);
    for column in ["", "id; --", "id, name", "id\0", "a.b.c"] {
        validation_error(AuditRecord::query().pluck_string(column).await);
        validation_error(AuditRecord::query().pluck_i32(column).await);
    }
    assert_eq!(
        AuditRecord::query()
            .order_by("id")
            .pluck_string("audit_records.name")
            .await
            .expect("qualified pluck"),
        vec!["alpha", "beta"]
    );
}

async fn empty_in_never_expands_reads_or_deletes_and_preserves_boolean_semantics() {
    let _guard = database().await;
    let mut tx = Orm::begin_transaction().await.expect("isolated deletion");
    for query in [
        AuditRecord::query().where_in("id", Vec::<i32>::new()),
        AuditRecord::query().or_where_in("id", Vec::<i32>::new()),
        AuditRecord::query()
            .where_eq("id", 1)
            .where_in("id", Vec::<i32>::new()),
    ] {
        assert!(
            query
                .get_with_tx(&mut tx)
                .await
                .expect("empty set read")
                .is_empty()
        );
        assert_eq!(
            query
                .delete_all_with_tx(&mut tx)
                .await
                .expect("empty set delete"),
            0
        );
    }
    for query in [
        AuditRecord::query()
            .where_eq("id", 1)
            .or_where_in("id", Vec::<i32>::new()),
        AuditRecord::query()
            .where_in("id", Vec::<i32>::new())
            .or_where("id", 1),
    ] {
        let rows = query
            .get_with_tx(&mut tx)
            .await
            .expect("false OR predicate");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 1);
    }
    assert_eq!(
        AuditRecord::query()
            .where_not_in("id", Vec::<i32>::new())
            .get_with_tx(&mut tx)
            .await
            .expect("NOT IN empty is true")
            .len(),
        2
    );
    assert_eq!(
        AuditRecord::all_with_tx(&mut tx)
            .await
            .expect("preserved records")
            .len(),
        2
    );
    tx.rollback().await.expect("rollback fixture");
}

async fn empty_in_bulk_delete_never_removes_a_row() {
    let _guard = database().await;
    for query in [
        AuditRecord::query().where_in("id", Vec::<i32>::new()),
        AuditRecord::query().or_where_in("id", Vec::<i32>::new()),
    ] {
        let mut tx = Orm::begin_transaction().await.expect("isolated deletion");
        assert_eq!(
            query
                .delete_all_with_tx(&mut tx)
                .await
                .expect("delete empty selection"),
            0
        );
        assert_eq!(
            AuditRecord::all_with_tx(&mut tx)
                .await
                .expect("preserved records")
                .len(),
            2
        );
        tx.rollback().await.expect("rollback fixture");
    }
}

async fn policy_protected_models_refuse_bulk_delete_without_per_row_authorization() {
    let _guard = database().await;
    let mut tx = Orm::begin_transaction()
        .await
        .expect("isolated policy deletion");
    let row = AuditProtectedRecord::find_with_tx(1, &mut tx)
        .await
        .expect("protected record")
        .expect("fixture");
    validation_error(row.delete_with_tx(&mut tx).await);
    validation_error(
        AuditProtectedRecord::query()
            .where_eq("id", 1)
            .delete_all_with_tx(&mut tx)
            .await,
    );
    assert_eq!(
        AuditRecord::all_with_tx(&mut tx)
            .await
            .expect("preserved protected rows")
            .len(),
        2
    );
    tx.rollback().await.expect("rollback fixture");
}

async fn global_and_tenant_scopes_remain_separate_from_user_or_filters() {
    let _guard = database().await;
    with_tenant("tenant-b", async {
        let query = AuditScopedRecord::query()
            .where_eq("id", 2)
            .or_where("id", 1);
        let rows = query.get().await.expect("composed scope read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 2);
        let nested = AuditRecord::query().where_exists(query);
        assert_eq!(
            nested
                .get()
                .await
                .expect("ordered nested scope bindings")
                .len(),
            2
        );
    })
    .await;
}

async fn tenant_scope_cannot_be_bypassed_by_or_filters() {
    let _guard = database().await;
    with_tenant("tenant-a", async {
        let query = AuditTenantRecord::query()
            .where_eq("name", "alpha")
            .or_where("id", 2);
        let rows = query.get().await.expect("tenant query");
        assert_eq!(rows.len(), 1, "OR must remain inside the tenant boundary");
        assert_eq!(rows[0].tenant_id, "tenant-a");
        assert_eq!(query.count().await.expect("tenant count"), 1);
        assert_eq!(query.pluck_i32("id").await.expect("tenant pluck"), vec![1]);
        let mut tx = Orm::begin_transaction().await.expect("isolated deletion");
        assert_eq!(
            query
                .delete_all_with_tx(&mut tx)
                .await
                .expect("scoped bulk delete"),
            1
        );
        let remaining = AuditTenantRecord::unscoped()
            .get_with_tx(&mut tx)
            .await
            .expect("remaining tenant");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].tenant_id, "tenant-b");
        tx.rollback().await.expect("rollback fixture");
    })
    .await;
}

async fn soft_delete_scope_applies_to_every_or_branch() {
    let _guard = database().await;
    let query = AuditSoftRecord::query().where_eq("id", 2).or_where("id", 1);
    let rows = query.get().await.expect("soft-delete filter");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "active");
    assert_eq!(query.count().await.expect("soft-delete count"), 1);
    assert_eq!(
        query.pluck_i32("id").await.expect("soft-delete pluck"),
        vec![1]
    );
    let deleted = query
        .clone()
        .only_trashed()
        .get()
        .await
        .expect("only deleted");
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].id, 2);
    assert_eq!(
        query
            .with_trashed()
            .get()
            .await
            .expect("explicit all rows")
            .len(),
        2
    );
}

async fn nested_subqueries_propagate_validation_and_missing_tenant_context() {
    let _guard = database().await;
    validation_error(
        AuditRecord::query()
            .where_exists(AuditTenantRecord::query())
            .get()
            .await,
    );
    validation_error(
        AuditRecord::query()
            .or_where_exists(AuditRecord::query().where_eq("id; --", 1))
            .get()
            .await,
    );
    validation_error(
        AuditRecord::query()
            .with_cte("scoped", AuditTenantRecord::query())
            .get()
            .await,
    );
    validation_error(
        AuditRecord::query()
            .with_recursive("scoped", AuditTenantRecord::query())
            .get()
            .await,
    );
}

async fn pluck_uses_the_active_transaction_and_observes_uncommitted_writes() {
    let _guard = database().await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Orm::transaction(|_| {
            Box::pin(async {
                let mut row = AuditRecord {
                    id: 0,
                    name: "inside transaction".to_string(),
                };
                row.save().await?;
                assert_eq!(
                    AuditRecord::query()
                        .where_eq("id", row.id)
                        .pluck_string("name")
                        .await?,
                    vec!["inside transaction"]
                );
                assert_eq!(
                    AuditRecord::query()
                        .where_eq("id", row.id)
                        .pluck_i32("id")
                        .await?,
                    vec![row.id]
                );
                Err::<(), Error>(Error::Validation("intentional audit rollback".to_string()))
            })
        }),
    )
    .await
    .expect(
        "pluck must not wait for a second pool connection while the transaction owns the only one",
    );
    assert!(
        matches!(result, Err(Error::DatabaseError(message)) if message.contains("intentional audit rollback"))
    );
    assert_eq!(
        AuditRecord::all().await.expect("read after rollback").len(),
        2
    );
}

async fn keyset_cursor_applies_to_all_or_branches() {
    let _guard = database().await;
    let query = AuditRecord::query().where_eq("id", 1).or_where("id", 2);
    let seen = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    query
        .chunk_by_id(1, {
            let seen = seen.clone();
            move |rows| unique_rows(seen.clone(), rows)
        })
        .await
        .expect("keyset traversal must advance through OR branches");
    assert_eq!(*seen.lock().await, vec![1, 2]);

    seen.lock().await.clear();
    let mut tx = Orm::begin_transaction().await.expect("keyset transaction");
    query
        .chunk_by_id_with_tx(1, &mut tx, {
            let seen = seen.clone();
            move |rows| unique_rows(seen.clone(), rows)
        })
        .await
        .expect("explicit keyset cursor must constrain every OR branch");
    assert_eq!(*seen.lock().await, vec![1, 2]);
    tx.rollback().await.expect("rollback keyset fixture");
}

async fn unique_rows(
    seen: std::sync::Arc<tokio::sync::Mutex<Vec<i32>>>,
    rows: Vec<AuditRecord>,
) -> Result<(), Error> {
    let mut seen = seen.lock().await;
    for row in rows {
        if seen.contains(&row.id) {
            return Err(Error::Validation("keyset repeated a row".to_string()));
        }
        seen.push(row.id);
    }
    Ok(())
}

async fn stream_uses_the_active_transaction() {
    use rullst_orm::_futures::TryStreamExt;
    let _guard = database().await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Orm::transaction(|_| {
            Box::pin(async {
                let mut row = AuditRecord {
                    id: 0,
                    name: "transaction stream".to_string(),
                };
                row.save().await?;
                let query = AuditRecord::query().where_eq("id", row.id);
                let rows: Vec<_> = query.stream().try_collect().await?;
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].name, "transaction stream");
                Err::<(), Error>(Error::Validation("intentional stream rollback".to_string()))
            })
        }),
    )
    .await
    .expect("stream must read its active transaction without acquiring another connection");
    assert!(
        matches!(result, Err(Error::DatabaseError(message)) if message.contains("intentional stream rollback"))
    );
    assert_eq!(AuditRecord::all().await.expect("stream rollback").len(), 2);
}

async fn managed_transactions_allow_reentrant_eager_loads_and_after_fetch_hooks() {
    let _guard = database().await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Orm::transaction(|_| {
            Box::pin(async {
                let mut role = AuditRole {
                    id: 0,
                    name: "transaction role".to_string(),
                };
                role.save().await?;
                rullst_orm::dispatch_executor!(pool, |executor| {
                    rullst_orm::_sqlx::query(
                        "INSERT INTO audit_parent_roles (parent_id, role_id) VALUES (?, ?)",
                    )
                    .bind(1)
                    .bind(role.id)
                    .execute(executor)
                    .await
                })?;
                let parents = AuditParent::query()
                    .with_children()
                    .with_roles()
                    .order_by("id")
                    .get()
                    .await?;
                assert_eq!(parents.len(), 2);
                assert_eq!(
                    parents[0].children.as_ref().expect("loaded children")[0].name,
                    "child alpha"
                );
                assert_eq!(AuditHookRecord::all().await?.len(), 2);
                assert_eq!(
                    parents[0].roles.as_ref().expect("transaction pivot read")[0].name,
                    "transaction role"
                );
                Err::<(), Error>(Error::Validation("intentional eager rollback".to_string()))
            })
        }),
    )
    .await
    .expect("fetch must release transaction lock before nested reads");
    assert!(
        matches!(result, Err(Error::DatabaseError(message)) if message.contains("intentional eager rollback"))
    );
}

async fn raw_transaction_reads_reject_unsupported_eager_and_hook_modes() {
    use rullst_orm::_futures::TryStreamExt;
    let _guard = database().await;
    let mut tx = Orm::begin_transaction()
        .await
        .expect("explicit raw transaction");
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        validation_error(
            AuditParent::query()
                .with_children()
                .get_with_tx(&mut tx)
                .await,
        );
        validation_error(AuditHookRecord::query().get_with_tx(&mut tx).await);
        validation_error(
            AuditHookRecord::query()
                .stream_with_tx(&mut tx)
                .try_collect::<Vec<_>>()
                .await,
        );
        assert_eq!(
            AuditRecord::all_with_tx(&mut tx)
                .await
                .expect("plain explicit read")
                .len(),
            2
        );
    })
    .await
    .expect("unsupported modes must fail before trying another executor");
    tx.rollback().await.expect("rollback explicit fixture");
}

// One runtime keeps the process-global in-memory SQLite pool alive across cases.
mod tests {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

    macro_rules! audit_case {
        ($name:ident) => {
            #[test]
            fn $name() {
                RUNTIME
                    .get_or_init(|| tokio::runtime::Runtime::new().expect("audit runtime"))
                    .block_on(super::$name());
            }
        };
    }

    audit_case!(safe_select_rejects_expressions_and_malformed_identifiers);
    audit_case!(safe_pluck_rejects_expressions_before_executing_sql);
    audit_case!(empty_in_never_expands_reads_or_deletes_and_preserves_boolean_semantics);
    audit_case!(empty_in_bulk_delete_never_removes_a_row);
    audit_case!(policy_protected_models_refuse_bulk_delete_without_per_row_authorization);
    audit_case!(global_and_tenant_scopes_remain_separate_from_user_or_filters);
    audit_case!(tenant_scope_cannot_be_bypassed_by_or_filters);
    audit_case!(soft_delete_scope_applies_to_every_or_branch);
    audit_case!(nested_subqueries_propagate_validation_and_missing_tenant_context);
    audit_case!(pluck_uses_the_active_transaction_and_observes_uncommitted_writes);
    audit_case!(keyset_cursor_applies_to_all_or_branches);
    audit_case!(stream_uses_the_active_transaction);
    audit_case!(managed_transactions_allow_reentrant_eager_loads_and_after_fetch_hooks);
    audit_case!(raw_transaction_reads_reject_unsupported_eager_and_hook_modes);
}
